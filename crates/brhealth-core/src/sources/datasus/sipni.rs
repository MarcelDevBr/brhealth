// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Programa Nacional de Imunizações (SI-PNI / RNDS - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, Date32Builder, StringBuilder, UInt8Builder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_date32_value, get_str_value, get_u8_value};
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

        // 1. vaccination_event_id (ID_DOSE ou índice)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "ID_DOSE", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("VAC_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. vaccine_code (COD_VACINA ou IMUNO)
        let mut code_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            let code = get_str_value(raw_batch, "COD_VACINA", i)
                .or_else(|| get_str_value(raw_batch, "IMUNO", i))
                .unwrap_or("00");
            code_builder.append_value(code);
        }
        let code_col: ArrayRef = Arc::new(code_builder.finish());

        // 3. vaccine_name (DS_VACINA ou NOME_VAC)
        let mut name_builder = StringBuilder::with_capacity(num_rows, num_rows * 20);
        for i in 0..num_rows {
            let name = get_str_value(raw_batch, "DS_VACINA", i)
                .or_else(|| get_str_value(raw_batch, "NOME_VAC", i))
                .unwrap_or("VACINA PADRAO SUS");
            name_builder.append_value(name);
        }
        let name_col: ArrayRef = Arc::new(name_builder.finish());

        // 4. dose_order (DOSE ou TP_DOSE)
        let mut dose_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            let dose = get_str_value(raw_batch, "DOSE", i)
                .or_else(|| get_str_value(raw_batch, "TP_DOSE", i))
                .unwrap_or("D1");
            dose_builder.append_value(dose);
        }
        let dose_col: ArrayRef = Arc::new(dose_builder.finish());

        // 5. vaccination_date (DT_VACINA ou DATA_APLIC)
        let mut date_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dt = get_date32_value(raw_batch, "DT_VACINA", i)
                .or_else(|| get_date32_value(raw_batch, "DATA_APLIC", i))
                .unwrap_or(0);
            date_builder.append_value(dt);
        }
        let date_col: ArrayRef = Arc::new(date_builder.finish());

        // 6. patient_municipality (MUN_RESID ou CODMUNRES)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "MUN_RESID", i)
                .or_else(|| get_str_value(raw_batch, "CODMUNRES", i))
                .and_then(|m| harmonize_ibge_code(m).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 7. vaccination_facility_cnes (CNES_ESTAB)
        let mut cnes_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let cnes = get_str_value(raw_batch, "CNES_ESTAB", i).unwrap_or("0000000");
            cnes_builder.append_value(cnes);
        }
        let cnes_col: ArrayRef = Arc::new(cnes_builder.finish());

        // 8. lot_number (LOTE)
        let mut lot_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            if let Some(lote) = get_str_value(raw_batch, "LOTE", i) {
                lot_builder.append_value(lote);
            } else {
                lot_builder.append_null();
            }
        }
        let lot_col: ArrayRef = Arc::new(lot_builder.finish());

        // 9. patient_age_years (IDADE)
        let mut age_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(age) = get_u8_value(raw_batch, "IDADE", i) {
                age_builder.append_value(age);
            } else {
                age_builder.append_null();
            }
        }
        let age_col: ArrayRef = Arc::new(age_builder.finish());

        // 10. patient_sex (SEXO)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows);
        for i in 0..num_rows {
            let s = get_str_value(raw_batch, "SEXO", i).unwrap_or("U");
            sex_builder.append_value(s);
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

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
