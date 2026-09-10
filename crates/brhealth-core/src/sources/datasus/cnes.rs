// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Cadastro Nacional de Estabelecimentos de Saúde (CNES - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_harmonized_ibge_col, build_null_col, build_str_col, build_u16_col, get_bool_value,
    get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o CNES (Estabelecimentos e Recursos Assistenciais) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct CnesDataSource;

use crate::domain::schema::default_values::DEFAULT_CNES;

impl CnesDataSource {
    /// Cria uma nova instância de `CnesDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de estabelecimentos.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_health_facility_schema();

        let cnes_id_col = build_str_col(raw_batch, "CNES", DEFAULT_CNES, num_rows);

        // facility_name (NOMEFANT ou RAZAOSOC)
        let name_col: ArrayRef = {
            let mut name_builder =
                arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 30);
            for i in 0..num_rows {
                let name = get_str_value(raw_batch, "NOMEFANT", i)
                    .or_else(|| get_str_value(raw_batch, "RAZAOSOC", i))
                    .unwrap_or("ESTABELECIMENTO DE SAUDE");
                name_builder.append_value(name);
            }
            Arc::new(name_builder.finish())
        };

        let mun_col = build_harmonized_ibge_col(raw_batch, "CODUFMUN", num_rows);
        let h3_col = build_null_col(&DataType::UInt64, num_rows);
        let mgmt_col = build_str_col(raw_batch, "TPGESTAO", "M", num_rows);
        let type_col = build_str_col(raw_batch, "TP_UNID", "00", num_rows);

        // has_emergency_care (ATEND_PRONTO_SOCORRO / ATENDAMB)
        let urg_col: ArrayRef = {
            let mut urg_builder = BooleanBuilder::with_capacity(num_rows);
            for i in 0..num_rows {
                let has_urg = get_bool_value(raw_batch, "ATEND_PRONTO_SOCORRO", i)
                    .or_else(|| get_bool_value(raw_batch, "ATENDURG", i))
                    .unwrap_or(false);
                urg_builder.append_value(has_urg);
            }
            Arc::new(urg_builder.finish())
        };

        let surg_col = build_u16_col(raw_batch, "QTLEITOCIRURGICO", 0, num_rows);
        let clin_col = build_u16_col(raw_batch, "QTLEITOCLINICO", 0, num_rows);
        let uti_sus_col = build_u16_col(raw_batch, "QTLEITOUTI_SUS", 0, num_rows);
        let uti_nonsus_col = build_u16_col(raw_batch, "QTLEITOUTI_NAOSUS", 0, num_rows);
        let comp_col = build_str_col(raw_batch, "COMPETEN", "202401", num_rows);

        RecordBatch::try_new(
            target_schema,
            vec![
                cnes_id_col,
                name_col,
                mun_col,
                h3_col,
                mgmt_col,
                type_col,
                urg_col,
                surg_col,
                clin_col,
                uti_sus_col,
                uti_nonsus_col,
                comp_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for CnesDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.cnes",
            display_name: "CNES - Cadastro Nacional de Estabelecimentos de Saúde",
            maintaining_agency: "DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::AssistanceInfrastructure,
            temporal_resolution: "Mensal",
            spatial_resolution: "Estabelecimento / Município (IBGE)",
            supported_years: 2005..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_health_facility_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let sub_type = params
            .extra_filters
            .get("type")
            .map(|s| s.as_str())
            .unwrap_or("ST"); // ST = Estabelecimentos, LT = Leitos
        let year_short = params.year % 100;
        let month = params.month.unwrap_or(1);
        let filename = format!(
            "{}{}{:02}{:02}.dbc",
            sub_type.to_uppercase(),
            uf.to_uppercase(),
            year_short,
            month
        );

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/CNES/200508_/Dados/{sub_type}/{filename}"
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
