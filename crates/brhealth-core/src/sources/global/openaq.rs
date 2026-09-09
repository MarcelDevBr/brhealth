// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI da Rede Global de Monitoramento da Qualidade do Ar (OpenAQ).

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

/// Adaptador SPI para a plataforma OpenAQ (Qualidade do Ar Global).
#[derive(Debug, Default, Clone)]
pub struct OpenAqDataSource;

impl OpenAqDataSource {
    /// Cria uma nova instância de `OpenAqDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthDataSourceSPI for OpenAqDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.openaq",
            display_name: "OpenAQ Global Air Quality Monitoring",
            maintaining_agency: "OpenAQ Community / EPA / EEA",
            scope: GeographicScope::GlobalGrid,
            category: SourceCategory::EnvironmentalPlanetary,
            temporal_resolution: "Horário e Diário",
            spatial_resolution: "Estação Terrestre / Célula H3 Res 8",
            supported_years: 2015..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_openaq_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let country = match &params.scope {
            GeographicScope::National { iso_3166_alpha3 } => iso_3166_alpha3.as_str(),
            _ => "BRA",
        };
        Ok(format!(
            "https://api.openaq.org/v2/measurements?country={country}&year={}",
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
