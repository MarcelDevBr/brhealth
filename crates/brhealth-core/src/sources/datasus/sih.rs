// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações Hospitalares (SIHSUS - DATASUS RD/AIH).

use std::sync::Arc;

use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::mapper::DatasusBatchHarmonizer;
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o SIHSUS (AIH Reduzida - RD) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SihDataSource;

use crate::domain::schema::default_values::PREFIX_AIH;

impl SihDataSource {
    /// Cria uma nova instância de `SihDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de morbidade hospitalar.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let target_schema = CanonicalSchemas::canonical_hospital_morbidity_schema();
        let harmonizer = DatasusBatchHarmonizer::new(target_schema)
            .with_record_id("N_AIH", PREFIX_AIH)
            .with_harmonized_ibge("MUNIC_RES")
            .with_harmonized_ibge("MUNIC_MOV")
            .with_date32("DT_INTER", 0)
            .with_date32("DT_SAIDA", 0)
            .with_u16("DIAS_PERM", 0)
            .with_str("DIAG_PRINC", "Z00")
            .with_str_opt("DIAG_SECUN", 5)
            .with_str("PROC_REA", "0000000000")
            .with_f64("VAL_TOT", 0.0)
            .with_u16("UTI_MES_TO", 0)
            .with_computed_bool("MORTE", "1")
            .with_constant_bool(false);

        harmonizer.harmonize(raw_batch)
    }
}

#[async_trait]
impl HealthDataSourceSPI for SihDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sih",
            display_name: "Sistema de Informações Hospitalares (SIHSUS RD/AIH - DATASUS)",
            maintaining_agency: "Ministério da Saúde (Brasil)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            category: SourceCategory::ClinicalMorbidity,
            temporal_resolution: "Mensal / Competência",
            spatial_resolution: "Municipal (IBGE 7 dígitos)",
            supported_years: 1998..=2024,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_hospital_morbidity_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year_2digits = params.year % 100;
        let month = params.month.unwrap_or(1);
        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
        ))
    }

    fn mirror_uris(&self, params: &DataQueryParams) -> Vec<String> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year_2digits = params.year % 100;
        let month = params.month.unwrap_or(1);
        vec![
            format!(
                "https://datasus.saude.gov.br/transferencia-download-de-arquivos/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
            ),
            format!(
                "ftp://ftp2.datasus.gov.br/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
            ),
        ]
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        let decompressed_dbf = context.decompressor.decompress(&raw_bytes)?;

        let dbf_decoder = DbfDecoder::new();
        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed_dbf)?;

        let canonical_batch = self.harmonize_batch(&raw_batch)?;

        Ok(vec![canonical_batch])
    }
}
