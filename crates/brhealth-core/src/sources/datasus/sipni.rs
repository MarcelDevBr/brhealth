// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Programa Nacional de Imunizações (SI-PNI / RNDS - DATASUS).

use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_record_id_col, build_sex_col, build_str_col, build_str_opt_col, build_u8_opt_col,
    get_date32_value, get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SI-PNI / RNDS Vacinas do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SipniDataSource;

use crate::domain::schema::default_values::{DEFAULT_CNES, DEFAULT_IBGE_MUNICIPALITY, PREFIX_VAC};

impl SipniDataSource {
    /// Cria uma nova instância de `SipniDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído para o schema canônico de vacinação.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_immunization_schema();

        let id_col = build_record_id_col(raw_batch, "ID_DOSE", PREFIX_VAC, num_rows);

        // vaccine_code (COD_VACINA ou IMUNO)
        let code_col: ArrayRef = {
            let mut code_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 6);
            for i in 0..num_rows {
                let code = get_str_value(raw_batch, "COD_VACINA", i)
                    .or_else(|| get_str_value(raw_batch, "IMUNO", i))
                    .unwrap_or("00");
                code_builder.append_value(code);
            }
            Arc::new(code_builder.finish())
        };

        // vaccine_name (DS_VACINA ou NOME_VAC)
        let name_col: ArrayRef = {
            let mut name_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 20);
            for i in 0..num_rows {
                let name = get_str_value(raw_batch, "DS_VACINA", i)
                    .or_else(|| get_str_value(raw_batch, "NOME_VAC", i))
                    .unwrap_or("VACINA PADRAO SUS");
                name_builder.append_value(name);
            }
            Arc::new(name_builder.finish())
        };

        // dose_order (DOSE ou TP_DOSE)
        let dose_col: ArrayRef = {
            let mut dose_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 4);
            for i in 0..num_rows {
                let dose = get_str_value(raw_batch, "DOSE", i)
                    .or_else(|| get_str_value(raw_batch, "TP_DOSE", i))
                    .unwrap_or("D1");
                dose_builder.append_value(dose);
            }
            Arc::new(dose_builder.finish())
        };

        // vaccination_date (DT_VACINA ou DATA_APLIC)
        let date_col: ArrayRef = {
            let mut date_builder = arrow::array::Date32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let dt = get_date32_value(raw_batch, "DT_VACINA", i)
                    .or_else(|| get_date32_value(raw_batch, "DATA_APLIC", i))
                    .unwrap_or(0);
                date_builder.append_value(dt);
            }
            Arc::new(date_builder.finish())
        };

        // patient_municipality (MUN_RESID ou CODMUNRES)
        let mun_col: ArrayRef = {
            let mut mun_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 7);
            for i in 0..num_rows {
                let resolved = get_str_value(raw_batch, "MUN_RESID", i)
                    .or_else(|| get_str_value(raw_batch, "CODMUNRES", i))
                    .and_then(|m| harmonize_ibge_code(m).ok())
                    .unwrap_or_else(|| DEFAULT_IBGE_MUNICIPALITY.to_string());
                mun_builder.append_value(resolved);
            }
            Arc::new(mun_builder.finish())
        };

        let cnes_col = build_str_col(raw_batch, "CNES_ESTAB", DEFAULT_CNES, num_rows);
        let lot_col = build_str_opt_col(raw_batch, "LOTE", 8, num_rows);
        let age_col = build_u8_opt_col(raw_batch, "IDADE", num_rows);
        let sex_col = build_sex_col(raw_batch, "SEXO", num_rows);

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, code_col, name_col, dose_col, date_col, mun_col, cnes_col, lot_col,
                age_col, sex_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SipniDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sipni",
            display_name: "SI-PNI / RNDS - Sistema de Informações do Programa Nacional de Imunizações",
            maintaining_agency: "DATASUS / CGPNI / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::VitalStatistics,
            temporal_resolution: "Diária / Mensal",
            spatial_resolution: "Município / Estabelecimento (CNES)",
            supported_years: 1994..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_immunization_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year_short = params.year % 100;
        let filename = format!("PNI{}{:02}.dbc", uf.to_uppercase(), year_short);

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/PNI/DADOS/{filename}"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;
        let dbf_bytes = context.decompressor.decompress(&raw_bytes)?;

        let decoder = DbfDecoder::new();
        let raw_batch = decoder.decode_to_record_batch(&dbf_bytes)?;
        let harmonized = self.harmonize_batch(&raw_batch)?;

        Ok(vec![harmonized])
    }
}
