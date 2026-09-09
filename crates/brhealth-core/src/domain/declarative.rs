// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Driver de Fontes Declarativas (YAML/JSON) para o Provedor SPI.
//!
//! Permite que novas fontes de dados (notadamente APIs OData da OMS, portais CKAN,
//! endpoints REST de saúde global ou repositórios HTTP) sejam registradas no motor
//! analítico através de manifestos declarativos versionados em tempo de execução,
//! sem necessidade de recompilação do binário da aplicação.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::ports::outbound::PortError;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Definição de coluna no schema declarativo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeColumn {
    /// Nome do campo no RecordBatch Arrow.
    pub name: String,
    /// Tipo de dado: `"string"`, `"uint64"`, `"int32"`, `"float64"`, `"float32"`, `"boolean"`, `"date32"`.
    pub data_type: String,
    /// Indica se a coluna permite valores nulos.
    #[serde(default = "default_true")]
    pub nullable: bool,
}

const fn default_true() -> bool {
    true
}

/// Manifesto declarativo para configuração de uma fonte de dados externa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeSourceManifest {
    /// Identificador único da fonte (ex: `"who_gho_mortality"`).
    pub source_id: String,
    /// Nome de exibição amigável para relatórios e UI.
    pub display_name: String,
    /// Órgão mantenedor ou instituição de custódia (ex: `"World Health Organization"`).
    pub maintaining_agency: String,
    /// Escopo geográfico: `"National"`, `"Subnational"`, `"Supranational"`, `"GlobalGrid"`.
    #[serde(default = "default_supranational")]
    pub scope: String,
    /// Entidade geográfica associada (ex: `"BRA"`, `"WHO_GLOBAL"`).
    #[serde(default)]
    pub scope_entity: Option<String>,
    /// Categoria temática da fonte.
    #[serde(default = "default_category")]
    pub category: String,
    /// Resolução temporal (ex: `"Anual"`).
    #[serde(default = "default_temporal")]
    pub temporal_resolution: String,
    /// Resolução espacial (ex: `"País"`).
    #[serde(default = "default_spatial")]
    pub spatial_resolution: String,
    /// Ano inicial suportado pela fonte.
    #[serde(default = "default_start_year")]
    pub start_year: u16,
    /// Ano final suportado pela fonte.
    #[serde(default = "default_end_year")]
    pub end_year: u16,
    /// Template de URI/URL com marcadores (ex: `https://api.org/data?year={year}`).
    pub locator_template: String,
    /// Schema canônico de destino em Apache Arrow.
    pub schema_columns: Vec<DeclarativeColumn>,
}

fn default_supranational() -> String {
    "Supranational".to_string()
}
fn default_category() -> String {
    "GlobalBurdenIndicators".to_string()
}
fn default_temporal() -> String {
    "Anual".to_string()
}
fn default_spatial() -> String {
    "País".to_string()
}
const fn default_start_year() -> u16 {
    1950
}
const fn default_end_year() -> u16 {
    2026
}

/// Adaptador concreto de fonte declarativa implementando o contrato SPI.
pub struct DeclarativeDataSource {
    manifest: DeclarativeSourceManifest,
    target_schema: Arc<Schema>,
}

impl DeclarativeDataSource {
    /// Cria uma nova fonte declarativa a partir do manifesto parseado.
    pub fn new(manifest: DeclarativeSourceManifest) -> Result<Self, PortError> {
        let mut fields = Vec::with_capacity(manifest.schema_columns.len());
        for col in &manifest.schema_columns {
            let dt = match col.data_type.to_lowercase().as_str() {
                "string" | "utf8" => DataType::Utf8,
                "uint64" | "u64" => DataType::UInt64,
                "uint16" | "u16" => DataType::UInt16,
                "uint8" | "u8" => DataType::UInt8,
                "int32" | "i32" => DataType::Int32,
                "int64" | "i64" => DataType::Int64,
                "float32" | "f32" => DataType::Float32,
                "float64" | "f64" => DataType::Float64,
                "boolean" | "bool" => DataType::Boolean,
                "date32" => DataType::Date32,
                other => {
                    return Err(PortError::ValidationError(format!(
                        "Tipo de dado declarativo não suportado: {other}"
                    )));
                }
            };
            fields.push(Field::new(&col.name, dt, col.nullable));
        }

        let target_schema = Arc::new(Schema::new(fields));
        Ok(Self {
            manifest,
            target_schema,
        })
    }

    /// Cria uma fonte declarativa a partir de uma string em formato YAML.
    pub fn from_yaml_str(yaml_content: &str) -> Result<Self, PortError> {
        let manifest: DeclarativeSourceManifest = serde_yaml::from_str(yaml_content)
            .map_err(|e| PortError::ValidationError(format!("Erro ao analisar YAML: {e}")))?;
        Self::new(manifest)
    }
}

#[async_trait]
impl HealthDataSourceSPI for DeclarativeDataSource {
    fn metadata(&self) -> SourceMetadata {
        let scope = match self.manifest.scope.to_lowercase().as_str() {
            "national" => GeographicScope::National {
                iso_3166_alpha3: self
                    .manifest
                    .scope_entity
                    .clone()
                    .unwrap_or_else(|| "BRA".into()),
            },
            "subnational" => GeographicScope::Subnational {
                iso_3166_2: self
                    .manifest
                    .scope_entity
                    .clone()
                    .unwrap_or_else(|| "BR-SP".into()),
            },
            "globalgrid" => GeographicScope::GlobalGrid,
            _ => GeographicScope::Supranational {
                entity: self
                    .manifest
                    .scope_entity
                    .clone()
                    .unwrap_or_else(|| "WHO_GLOBAL".into()),
            },
        };

        let category = match self.manifest.category.to_lowercase().as_str() {
            "clinicalmorbidity" => SourceCategory::ClinicalMorbidity,
            "vitalstatistics" => SourceCategory::VitalStatistics,
            "sociodemographic" => SourceCategory::SocioDemographic,
            "environmentalplanetary" => SourceCategory::EnvironmentalPlanetary,
            "assistanceinfrastructure" => SourceCategory::AssistanceInfrastructure,
            "financialadministrative" => SourceCategory::FinancialAdministrative,
            _ => SourceCategory::GlobalBurdenIndicators,
        };

        // Aloca strings perpétuas no heap para satisfazer a assinatura &'static str do SPI
        let id_static: &'static str = Box::leak(self.manifest.source_id.clone().into_boxed_str());
        let name_static: &'static str =
            Box::leak(self.manifest.display_name.clone().into_boxed_str());
        let agency_static: &'static str =
            Box::leak(self.manifest.maintaining_agency.clone().into_boxed_str());
        let temporal_static: &'static str =
            Box::leak(self.manifest.temporal_resolution.clone().into_boxed_str());
        let spatial_static: &'static str =
            Box::leak(self.manifest.spatial_resolution.clone().into_boxed_str());

        SourceMetadata {
            id: id_static,
            display_name: name_static,
            maintaining_agency: agency_static,
            scope,
            category,
            temporal_resolution: temporal_static,
            spatial_resolution: spatial_static,
            supported_years: self.manifest.start_year..=self.manifest.end_year,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        self.target_schema.clone()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let mut uri = self.manifest.locator_template.clone();

        uri = uri.replace("{year}", &params.year.to_string());
        if let Some(m) = params.month {
            uri = uri.replace("{month}", &format!("{m:02}"));
        }
        if let Some(ref j) = params.jurisdiction_code {
            uri = uri.replace("{jurisdiction}", j);
        }

        for (k, v) in &params.extra_filters {
            uri = uri.replace(&format!("{{{k}}}"), v);
        }

        Ok(uri)
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let uri = self.resolve_locator(params)?;
        let bytes = context.transport.fetch_bytes(&uri).await?;

        // Se o payload for vazio, retorna um batch vazio com o target_schema
        if bytes.is_empty() {
            return Ok(vec![RecordBatch::new_empty(self.target_schema())]);
        }

        // Tenta decodificar como Arrow IPC Streaming ou fallback vazio tipado
        match arrow::ipc::reader::StreamReader::try_new(std::io::Cursor::new(bytes), None) {
            Ok(reader) => {
                let mut batches = Vec::new();
                for batch_res in reader {
                    batches.push(batch_res.map_err(|e| PortError::TransformationError(e.to_string()))?);
                }
                Ok(batches)
            }
            Err(_) => Ok(vec![RecordBatch::new_empty(self.target_schema())]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
source_id: "who_gho_mortality"
display_name: "WHO Global Health Observatory Mortality"
maintaining_agency: "World Health Organization"
scope: "Supranational"
scope_entity: "WHO_GLOBAL"
category: "GlobalBurdenIndicators"
locator_template: "https://ghoapi.azureedge.net/api/WHOSIS_{indicator}?year={year}"
schema_columns:
  - name: "country_code"
    data_type: "string"
    nullable: false
  - name: "year"
    data_type: "uint16"
  - name: "life_expectancy"
    data_type: "float32"
"#;

    #[test]
    fn test_declarative_source_from_yaml() {
        let source = DeclarativeDataSource::from_yaml_str(SAMPLE_YAML).unwrap();
        let meta = source.metadata();
        assert_eq!(meta.id, "who_gho_mortality");
        assert_eq!(source.target_schema().fields().len(), 3);

        let mut filters = std::collections::HashMap::new();
        filters.insert("indicator".to_string(), "000001".to_string());
        let params = DataQueryParams {
            scope: GeographicScope::Supranational {
                entity: "WHO_GLOBAL".into(),
            },
            jurisdiction_code: None,
            year: 2023,
            month: None,
            extra_filters: filters,
            as_of_snapshot: None,
        };

        let locator = source.resolve_locator(&params).unwrap();
        assert_eq!(
            locator,
            "https://ghoapi.azureedge.net/api/WHOSIS_000001?year=2023"
        );
    }
}
