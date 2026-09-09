// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI da Pesquisa de Informações Básicas Municipais (IBGE MUNIC).

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

/// Adaptador SPI para a Pesquisa de Informações Básicas Municipais (IBGE MUNIC).
#[derive(Debug, Default, Clone)]
pub struct IbgeMunicDataSource;

impl IbgeMunicDataSource {
    /// Cria uma nova instância de `IbgeMunicDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HealthDataSourceSPI for IbgeMunicDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "ibge.munic",
            display_name: "Pesquisa de Informações Básicas Municipais (MUNIC)",
            maintaining_agency: "Instituto Brasileiro de Geografia e Estatística (IBGE)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::AssistanceInfrastructure,
            temporal_resolution: "Bienal",
            spatial_resolution: "Município (5570 municípios)",
            supported_years: 1999..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_munic_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        Ok(format!(
            "https://ftp.ibge.gov.br/Perfil_Municipios/MUNIC_{}/Base_de_Dados.zip",
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
