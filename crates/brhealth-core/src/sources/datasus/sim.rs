// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações sobre Mortalidade (SIM - DATASUS).

use std::sync::Arc;

use arrow::datatypes::{DataType, Schema};
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

/// Adaptador SPI para o SIM (Mortalidade Geral) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SimDataSource;

use crate::domain::schema::default_values::PREFIX_SIM;

impl SimDataSource {
    /// Cria uma nova instância de `SimDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de mortalidade.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let target_schema = CanonicalSchemas::canonical_mortality_schema();
        let harmonizer = DatasusBatchHarmonizer::new(target_schema)
            .with_record_id("NUMERODO", PREFIX_SIM)
            .with_constant_str("BRA")
            .with_harmonized_ibge("CODMUNRES")
            .with_null(DataType::UInt64)
            .with_date32("DTOBITO", 0)
            .with_str("CAUSABAS", "R99")
            .with_null(DataType::Utf8)
            .with_sim_age("IDADE")
            .with_sex("SEXO")
            .with_race("RACACOR")
            .with_computed_bool("OBITOMATER", "1");

        harmonizer.harmonize(raw_batch)
    }
}

#[async_trait]
impl HealthDataSourceSPI for SimDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sim",
            display_name: "Sistema de Informações sobre Mortalidade (SIM/DATASUS)",
            maintaining_agency: "Ministério da Saúde (Brasil)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            category: SourceCategory::VitalStatistics,
            temporal_resolution: "Diária / Anual",
            spatial_resolution: "Municipal (IBGE 7 dígitos)",
            supported_years: 1979..=2024,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_mortality_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"
        ))
    }

    fn mirror_uris(&self, params: &DataQueryParams) -> Vec<String> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        vec![
            format!(
                "https://datasus.saude.gov.br/transferencia-download-de-arquivos/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"
            ),
            format!("ftp://ftp2.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"),
        ]
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        // Descompressão do arquivo .dbc
        let decompressed_dbf = context.decompressor.decompress(&raw_bytes)?;

        // Decodificação DBF para RecordBatch bruto
        let dbf_decoder = DbfDecoder::new();
        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed_dbf)?;

        // Harmonização para o schema canônico
        let canonical_batch = self.harmonize_batch(&raw_batch)?;

        Ok(vec![canonical_batch])
    }
}
