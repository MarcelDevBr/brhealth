// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Junções Espaciais Vetorizadas em Memória Contígua (Apache Arrow).
//!
//! Executa *Spatial Joins* colunares de altíssimo desempenho baseados em chaves discretas
//! inteiras `UInt64` (células hexagonais Uber H3 ou quadrículas Google S2), cruzando
//! eventos de saúde com matrizes climáticas (ERA5 / INMET) sem intersecções geométricas lentas.

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{Array, AsArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};

use crate::domain::ports::outbound::PortError;

/// Executa um Spatial Join interno (*Inner Join*) entre dois lotes colunares baseado em chave espacial `UInt64`.
///
/// Cruza, por exemplo, registros de notificação de arboviroses com séries climáticas
/// na mesma célula e semana.
pub fn spatial_join_on_index(
    left_batch: &RecordBatch,
    right_batch: &RecordBatch,
    index_col_name: &str,
) -> Result<RecordBatch, PortError> {
    let left_col = left_batch.column_by_name(index_col_name).ok_or_else(|| {
        PortError::ValidationError(format!(
            "Coluna de índice '{index_col_name}' não encontrada no lote esquerdo"
        ))
    })?;
    let right_col = right_batch.column_by_name(index_col_name).ok_or_else(|| {
        PortError::ValidationError(format!(
            "Coluna de índice '{index_col_name}' não encontrada no lote direito"
        ))
    })?;

    if left_col.data_type() != &DataType::UInt64 || right_col.data_type() != &DataType::UInt64 {
        return Err(PortError::ValidationError(
            "Spatial Join discreto requer coluna de chave do tipo UInt64 (H3 ou S2)".into(),
        ));
    }

    let left_keys = left_col.as_primitive::<arrow::datatypes::UInt64Type>();
    let right_keys = right_col.as_primitive::<arrow::datatypes::UInt64Type>();

    // Constrói hash index do lote direito: chave -> linha do lote direito
    let mut right_index: HashMap<u64, usize> = HashMap::with_capacity(right_batch.num_rows());
    for i in 0..right_batch.num_rows() {
        if right_keys.is_valid(i) {
            right_index.insert(right_keys.value(i), i);
        }
    }

    // Identifica linhas correspondentes
    let mut matched_left_indices = Vec::new();
    let mut matched_right_indices = Vec::new();

    for i in 0..left_batch.num_rows() {
        if left_keys.is_valid(i) {
            let key = left_keys.value(i);
            if let Some(&right_idx) = right_index.get(&key) {
                matched_left_indices.push(i);
                matched_right_indices.push(right_idx);
            }
        }
    }

    if matched_left_indices.is_empty() {
        // Retorna schema unificado vazio
        let mut fields: Vec<Arc<Field>> = left_batch.schema().fields().to_vec();
        for f in right_batch.schema().fields() {
            if f.name() != index_col_name && !fields.iter().any(|existing| existing.name() == f.name()) {
                fields.push(f.clone());
            }
        }
        let empty_schema = Arc::new(Schema::new(fields));
        return Ok(RecordBatch::new_empty(empty_schema));
    }

    // Projeta colunas do lote esquerdo filtradas
    let left_take_indices = arrow::array::UInt32Array::from(
        matched_left_indices
            .iter()
            .map(|&idx| u32::try_from(idx).unwrap_or(0))
            .collect::<Vec<u32>>(),
    );
    let right_take_indices = arrow::array::UInt32Array::from(
        matched_right_indices
            .iter()
            .map(|&idx| u32::try_from(idx).unwrap_or(0))
            .collect::<Vec<u32>>(),
    );

    let mut result_fields: Vec<Arc<Field>> = left_batch.schema().fields().to_vec();
    let mut result_columns: Vec<Arc<dyn Array>> = Vec::new();

    for col in left_batch.columns() {
        let taken = arrow::compute::take(col.as_ref(), &left_take_indices, None)
            .map_err(|e| PortError::TransformationError(e.to_string()))?;
        result_columns.push(taken);
    }

    // Projeta colunas complementares do lote direito (exceto a chave espacial já presente)
    for (i, f) in right_batch.schema().fields().iter().enumerate() {
        if f.name() != index_col_name && !result_fields.iter().any(|existing| existing.name() == f.name()) {
            result_fields.push(f.clone());
            let right_col = right_batch.column(i);
            let taken = arrow::compute::take(right_col.as_ref(), &right_take_indices, None)
                .map_err(|e| PortError::TransformationError(e.to_string()))?;
            result_columns.push(taken);
        }
    }

    let result_schema = Arc::new(Schema::new(result_fields));
    RecordBatch::try_new(result_schema, result_columns).map_err(|e| PortError::TransformationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float32Array, StringArray, UInt64Array};

    #[test]
    fn test_spatial_join_on_index() {
        let left_schema = Arc::new(Schema::new(vec![
            Field::new("h3_index", DataType::UInt64, false),
            Field::new("disease", DataType::Utf8, false),
        ]));
        let left_keys = Arc::new(UInt64Array::from(vec![100, 200, 300]));
        let left_diseases = Arc::new(StringArray::from(vec!["DENG", "CHIK", "ZIKA"]));
        let left = RecordBatch::try_new(left_schema, vec![left_keys, left_diseases]).unwrap();

        let right_schema = Arc::new(Schema::new(vec![
            Field::new("h3_index", DataType::UInt64, false),
            Field::new("temp_c", DataType::Float32, false),
        ]));
        let right_keys = Arc::new(UInt64Array::from(vec![200, 300, 400]));
        let right_temps = Arc::new(Float32Array::from(vec![28.5, 30.2, 22.1]));
        let right = RecordBatch::try_new(right_schema, vec![right_keys, right_temps]).unwrap();

        let joined = spatial_join_on_index(&left, &right, "h3_index").unwrap();
        assert_eq!(joined.num_rows(), 2); // 200 e 300 coincidem
        assert_eq!(joined.num_columns(), 3); // h3_index, disease, temp_c
        assert_eq!(joined.schema().field(2).name(), "temp_c");
    }
}
