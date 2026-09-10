// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações Ambulatoriais (SIASUS - DATASUS BPA/APAC).

use std::sync::Arc;

use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_date32_col, build_f64_col, build_harmonized_ibge_col, build_record_id_col, build_str_col,
    build_str_opt_col, build_u16_opt_col, build_u32_col,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

use crate::domain::schema::default_values::{DEFAULT_CNES, DEFAULT_PROCEDURE_SIGTAP, PREFIX_AMB};

/// Adaptador SPI para o SIASUS (Produção Ambulatorial) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SiasusDataSource;

impl SiasusDataSource {
    /// Cria uma nova instância de `SiasusDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico ambulatorial.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_ambulatory_schema();

        let record_id_col = build_record_id_col(raw_batch, "PA_DOC_ID", PREFIX_AMB, num_rows);
        let patient_mun_col = build_harmonized_ibge_col(raw_batch, "PA_MUNPCN", num_rows);
        let cnes_col = build_str_col(raw_batch, "PA_CODUNI", DEFAULT_CNES, num_rows);
        let fac_mun_col = build_harmonized_ibge_col(raw_batch, "PA_UFMUN", num_rows);
        let proc_col = build_str_col(raw_batch, "PA_PROC_ID", DEFAULT_PROCEDURE_SIGTAP, num_rows);
        let date_col = build_date32_col(raw_batch, "PA_CMP", 0, num_rows);
        let cid_col = build_str_opt_col(raw_batch, "PA_CIDPRI", 5, num_rows);
        let qty_col = build_u32_col(raw_batch, "PA_QTDPRO", 1, num_rows);
        let cost_col = build_f64_col(raw_batch, "PA_VALPRO", 0.0, num_rows);
        let sex_col = build_str_opt_col(raw_batch, "PA_SEXO", 2, num_rows);
        let age_col = build_u16_opt_col(raw_batch, "PA_IDADE", num_rows);

        RecordBatch::try_new(
            target_schema,
            vec![
                record_id_col,
                patient_mun_col,
                cnes_col,
                fac_mun_col,
                proc_col,
                date_col,
                cid_col,
                qty_col,
                cost_col,
                sex_col,
                age_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SiasusDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.siasus",
            display_name: "SIASUS - Sistema de Informações Ambulatoriais do SUS",
            maintaining_agency: "DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::AssistanceInfrastructure,
            temporal_resolution: "Mensal",
            spatial_resolution: "Município / Estabelecimento (CNES)",
            supported_years: 1994..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_ambulatory_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let sub_system = params
            .extra_filters
            .get("sub_system")
            .map(|s| s.as_str())
            .unwrap_or("PA"); // PA = Produção Ambulatorial, APAC = Alta Complexidade
        let year_short = params.year % 100;
        let month = params.month.unwrap_or(1);
        let filename = format!(
            "{}{}{:02}{:02}.dbc",
            sub_system.to_uppercase(),
            uf.to_uppercase(),
            year_short,
            month
        );

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIASUS/200801_/Dados/{filename}"
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
