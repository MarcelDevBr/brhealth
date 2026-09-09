// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI da Pesquisa Nacional de Saúde do Escolar (IBGE PeNSE).

use std::sync::Arc;

use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para a Pesquisa Nacional de Saúde do Escolar (IBGE PeNSE).
#[derive(Debug, Default, Clone)]
pub struct IbgePenseDataSource;

impl IbgePenseDataSource {
    /// Cria uma nova instância de `IbgePenseDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthDataSourceSPI for IbgePenseDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "ibge.pense",
            display_name: "Pesquisa Nacional de Saúde do Escolar (PeNSE)",
            maintaining_agency: "IBGE / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Trienal",
            spatial_resolution: "Aluno / Escola / Município",
            supported_years: 2009..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_pense_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        Ok(format!(
            "https://ftp.ibge.gov.br/pense/pense_{}/microdados/dados.zip",
            params.year
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let uri = self.resolve_locator(params)?;
        let bytes = context.transport.fetch_bytes(&uri).await?;
        if bytes.is_empty() {
            return Ok(vec![RecordBatch::new_empty(self.target_schema())]);
        }
        Ok(vec![RecordBatch::new_empty(self.target_schema())])
    }
}
