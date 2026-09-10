// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Pipeline de Transformação Colunar Desacoplado (*Chain of Responsibility / Middleware*).
//!
//! Permite o encadeamento de transformações atômicas e puras sobre `arrow::record_batch::RecordBatch`
//! (como indexação espacial H3, enriquecimento de CSAP e harmonização de códigos IBGE),
//! respeitando o Princípio Aberto/Fechado (OCP) e operando em memória contígua alinhada a 64 bytes.

use std::fmt::Debug;
use std::sync::Arc;

use arrow::array::{Array, StringArray};
use arrow::record_batch::RecordBatch;

use crate::domain::ports::outbound::PortError;
use crate::domain::spatial::h3::append_h3_column;
use crate::domain::transforms::ibge::harmonize_ibge_code_to_buf;

/// Contrato para um passo de transformação colunar pura sobre um `RecordBatch`.
pub trait BatchTransformationStep: Send + Sync + Debug {
    /// Nome identificador do passo para telemetria e diagnóstico.
    fn name(&self) -> &'static str;

    /// Aplica a transformação sobre o lote colunar de dados.
    fn apply(&self, batch: RecordBatch) -> Result<RecordBatch, PortError>;
}

/// Passo de indexação espacial discreta Uber H3.
#[derive(Debug, Clone)]
pub struct H3SpatialIndexingStep {
    pub lat_col: String,
    pub lon_col: String,
    pub out_col: String,
    pub resolution: u8,
}

impl H3SpatialIndexingStep {
    /// Cria um novo passo de indexação H3.
    #[must_use]
    pub fn new(
        lat_col: impl Into<String>,
        lon_col: impl Into<String>,
        out_col: impl Into<String>,
        resolution: u8,
    ) -> Self {
        Self {
            lat_col: lat_col.into(),
            lon_col: lon_col.into(),
            out_col: out_col.into(),
            resolution,
        }
    }
}

impl BatchTransformationStep for H3SpatialIndexingStep {
    fn name(&self) -> &'static str {
        "H3SpatialIndexingStep"
    }

    fn apply(&self, batch: RecordBatch) -> Result<RecordBatch, PortError> {
        if batch.column_by_name(&self.lat_col).is_some()
            && batch.column_by_name(&self.lon_col).is_some()
        {
            append_h3_column(
                &batch,
                &self.lat_col,
                &self.lon_col,
                &self.out_col,
                self.resolution,
            )
        } else {
            Ok(batch)
        }
    }
}

/// Passo de enriquecimento clínico e econômico de morbidade hospitalar (CSAP).
#[derive(Debug, Clone, Default)]
pub struct CsapEnrichmentStep;

impl CsapEnrichmentStep {
    /// Cria uma nova instância de `CsapEnrichmentStep`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl BatchTransformationStep for CsapEnrichmentStep {
    fn name(&self) -> &'static str {
        "CsapEnrichmentStep"
    }

    fn apply(&self, batch: RecordBatch) -> Result<RecordBatch, PortError> {
        if batch.column_by_name("main_diagnosis_icd10").is_some() {
            crate::domain::analytics::csap::enrich_sih_batch_with_csap(&batch)
        } else {
            Ok(batch)
        }
    }
}

/// Passo de harmonização de colunas de municípios para 7 dígitos canônicos do IBGE.
#[derive(Debug, Clone)]
pub struct IbgeHarmonizationStep {
    pub municipality_columns: Vec<String>,
}

impl IbgeHarmonizationStep {
    /// Cria um novo passo para harmonizar os municípios das colunas especificadas.
    #[must_use]
    pub fn new(municipality_columns: Vec<String>) -> Self {
        Self {
            municipality_columns,
        }
    }
}

impl BatchTransformationStep for IbgeHarmonizationStep {
    fn name(&self) -> &'static str {
        "IbgeHarmonizationStep"
    }

    fn apply(&self, batch: RecordBatch) -> Result<RecordBatch, PortError> {
        if self.municipality_columns.is_empty() {
            return Ok(batch);
        }

        let num_rows = batch.num_rows();
        let mut new_columns = batch.columns().to_vec();
        let schema = batch.schema();

        for col_name in &self.municipality_columns {
            if let Ok(idx) = schema.index_of(col_name) {
                let col = batch.column(idx);
                if let Some(str_col) = col.as_any().downcast_ref::<StringArray>() {
                    let mut builder =
                        arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 7);
                    for i in 0..num_rows {
                        if str_col.is_valid(i) {
                            let val = str_col.value(i).trim();
                            if let Ok(buf) = harmonize_ibge_code_to_buf(val)
                                && let Ok(s) = std::str::from_utf8(&buf)
                            {
                                builder.append_value(s);
                                continue;
                            }
                        }
                        builder.append_value(
                            crate::domain::schema::default_values::DEFAULT_IBGE_MUNICIPALITY,
                        );
                    }
                    new_columns[idx] = Arc::new(builder.finish());
                }
            }
        }

        RecordBatch::try_new(schema, new_columns)
            .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

/// Passo de transformação customizado encapsulado via closure pura.
pub struct CustomTransformationStep {
    name: &'static str,
    action: Arc<dyn Fn(RecordBatch) -> Result<RecordBatch, PortError> + Send + Sync>,
}

impl CustomTransformationStep {
    /// Cria um novo passo customizado.
    pub fn new<F>(name: &'static str, action: F) -> Self
    where
        F: Fn(RecordBatch) -> Result<RecordBatch, PortError> + Send + Sync + 'static,
    {
        Self {
            name,
            action: Arc::new(action),
        }
    }
}

impl Debug for CustomTransformationStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomTransformationStep")
            .field("name", &self.name)
            .finish()
    }
}

impl BatchTransformationStep for CustomTransformationStep {
    fn name(&self) -> &'static str {
        self.name
    }

    fn apply(&self, batch: RecordBatch) -> Result<RecordBatch, PortError> {
        (self.action)(batch)
    }
}

/// Pipeline encadeado de transformações colunares (*Chain of Responsibility*).
#[derive(Debug, Default, Clone)]
pub struct TransformationPipeline {
    steps: Vec<Arc<dyn BatchTransformationStep>>,
}

impl TransformationPipeline {
    /// Constrói um pipeline vazio de transformações.
    #[must_use]
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Adiciona um passo ao final da cadeia de execução.
    #[must_use]
    pub fn add_step<S: BatchTransformationStep + 'static>(mut self, step: S) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    /// Adiciona um passo compartilhado `Arc<dyn BatchTransformationStep>` à cadeia.
    #[must_use]
    pub fn add_shared_step(mut self, step: Arc<dyn BatchTransformationStep>) -> Self {
        self.steps.push(step);
        self
    }

    /// Quantidade de passos configurados no pipeline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Verifica se o pipeline não possui passos configurados.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Executa toda a cadeia de transformações sequencialmente sobre um `RecordBatch`.
    pub fn execute(&self, mut batch: RecordBatch) -> Result<RecordBatch, PortError> {
        for step in &self.steps {
            batch = step.apply(batch)?;
        }
        Ok(batch)
    }

    /// Executa a cadeia de transformações sequencialmente sobre uma coleção de `RecordBatch`.
    pub fn execute_batches(
        &self,
        batches: Vec<RecordBatch>,
    ) -> Result<Vec<RecordBatch>, PortError> {
        batches.into_iter().map(|b| self.execute(b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    fn create_test_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("code", DataType::Utf8, false),
            Field::new("lat", DataType::Float64, false),
            Field::new("lon", DataType::Float64, false),
        ]));

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["355030", "330455"])),
                Arc::new(Float64Array::from(vec![-23.5505, -22.9068])),
                Arc::new(Float64Array::from(vec![-46.6333, -43.1729])),
            ],
        )
        .expect("batch creation failed")
    }

    #[test]
    fn test_pipeline_chain_execution() {
        let batch = create_test_batch();
        let pipeline = TransformationPipeline::new()
            .add_step(IbgeHarmonizationStep::new(vec!["code".into()]))
            .add_step(H3SpatialIndexingStep::new("lat", "lon", "h3_res8", 8));

        assert_eq!(pipeline.len(), 2);
        let transformed = pipeline.execute(batch).expect("execution failed");

        // Verifica código harmonizado de 6 para 7 dígitos
        let code_col = transformed
            .column_by_name("code")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .expect("code col missing");
        assert_eq!(code_col.value(0), "3550308"); // SP com DV 8
        assert_eq!(code_col.value(1), "3304557"); // RJ com DV 7

        // Verifica coluna H3 gerada
        assert!(transformed.column_by_name("h3_res8").is_some());
    }

    #[test]
    fn test_custom_step_in_pipeline() {
        let batch = create_test_batch();
        let pipeline = TransformationPipeline::new().add_step(CustomTransformationStep::new(
            "AddDummyColumn",
            |b| {
                let mut fields = b.schema().fields().to_vec();
                fields.push(Arc::new(Field::new("dummy", DataType::Utf8, false)));
                let mut cols = b.columns().to_vec();
                cols.push(Arc::new(StringArray::from(vec!["X", "Y"])));
                RecordBatch::try_new(Arc::new(Schema::new(fields)), cols)
                    .map_err(|e| PortError::TransformationError(e.to_string()))
            },
        ));

        let transformed = pipeline.execute(batch).expect("execution failed");
        assert!(transformed.column_by_name("dummy").is_some());
    }
}
