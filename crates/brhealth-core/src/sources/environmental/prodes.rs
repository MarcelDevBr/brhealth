// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Monitoramento de Desmatamento da Amazônia e Demais Biomas (INPE PRODES).

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

/// Adaptador SPI para o Programa de Monitoramento do Desmatamento da Amazônia Legal por Satélite (INPE PRODES).
#[derive(Debug, Default, Clone)]
pub struct ProdesDataSource;

impl ProdesDataSource {
    /// Cria uma nova instância de `ProdesDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthDataSourceSPI for ProdesDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "environmental.prodes",
            display_name: "Monitoramento de Desmatamento por Satélite (PRODES / INPE)",
            maintaining_agency: "Instituto Nacional de Pesquisas Espaciais (INPE / MCTI)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::EnvironmentalPlanetary,
            temporal_resolution: "Anual",
            spatial_resolution: "Polígonos / Células H3 / Municípios",
            supported_years: 1988..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_prodes_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        Ok(format!(
            "http://terrabrasilis.dpi.inpe.br/download/dataset/prodes-legal-amz/yearly/deforestation_{}.parquet",
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
