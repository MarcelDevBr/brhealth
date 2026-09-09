// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Indexação espacial discreta utilizando a malha hexagonal Uber H3.
//!
//! Permite indexar eventos e determinantes socioambientais em células hexagonais
//! unívocas representadas por inteiros de 64 bits (`uint64`), acelerando consultas
//! de vizinhança espacial, agregação geográfica e detecção de clusters epidemiológicos.

use std::sync::Arc;

use arrow::array::{Array, ArrayRef, Float64Array, UInt64Builder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use h3o::{CellIndex, LatLng, Resolution};

use crate::domain::ports::outbound::PortError;

/// Resolução H3 recomendada para análises municipais e climatológicas (~1,2 km de raio médio).
pub const DEFAULT_MUNICIPAL_RESOLUTION: u8 = 7;

/// Resolução H3 recomendada para vigilância epidemiológica intraurbana (~460 m de raio médio).
pub const DEFAULT_INTRAURBAN_RESOLUTION: u8 = 8;

/// Resolução H3 de alta precisão para focos de transmissão e vetores (~174 m de raio médio).
pub const DEFAULT_HIGH_PRECISION_RESOLUTION: u8 = 9;

/// Converte uma coordenada geográfica (latitude, longitude) em WGS84 para um índice H3 de 64 bits.
///
/// # Exemplos
///
/// ```rust
/// use brhealth_core::domain::spatial::h3::coord_to_h3_index;
///
/// // Marco Zero de São Paulo (Praça da Sé)
/// let lat = -23.550520;
/// let lng = -46.633308;
/// let h3_res8 = coord_to_h3_index(lat, lng, 8).unwrap();
/// assert_ne!(h3_res8, 0);
/// ```
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se as coordenadas estiverem fora dos limites do globo
/// ou se a resolução estiver fora do intervalo permitido de 0 a 15.
pub fn coord_to_h3_index(lat: f64, lng: f64, resolution: u8) -> Result<u64, PortError> {
    let res = Resolution::try_from(resolution).map_err(|e| {
        PortError::ValidationError(format!("Resolução H3 inválida ({resolution}): {e}"))
    })?;

    let lat_lng = LatLng::new(lat, lng).map_err(|e| {
        PortError::ValidationError(format!(
            "Coordenadas inválidas (lat: {lat}, lng: {lng}): {e}"
        ))
    })?;

    let cell = lat_lng.to_cell(res);
    Ok(u64::from(cell))
}

/// Converte um índice de célula H3 de 64 bits para o centróide em coordenadas WGS84 (latitude, longitude).
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se o valor `h3_index` não representar uma célula H3 válida.
pub fn h3_index_to_coord(h3_index: u64) -> Result<(f64, f64), PortError> {
    let cell = CellIndex::try_from(h3_index).map_err(|e| {
        PortError::ValidationError(format!("Índice H3 inválido (0x{h3_index:x}): {e}"))
    })?;

    let lat_lng = LatLng::from(cell);
    Ok((lat_lng.lat(), lat_lng.lng()))
}

/// Retorna as células vizinhas em um disco espacial de raio $k$ (anel/vizinhança de ordem $k$).
///
/// Útil para modelar zonas de amortecimento (buffers) e difusão de contágio ou arboviroses.
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se o índice H3 for inválido.
pub fn h3_grid_disk(h3_index: u64, k: u32) -> Result<Vec<u64>, PortError> {
    let cell = CellIndex::try_from(h3_index).map_err(|e| {
        PortError::ValidationError(format!("Índice H3 inválido (0x{h3_index:x}): {e}"))
    })?;

    let neighbors = cell.grid_disk::<Vec<_>>(k);
    Ok(neighbors.into_iter().map(u64::from).collect())
}

/// Calcula a distância de grade em número de células hexagonais entre duas posições H3 de mesma resolução.
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se algum dos índices for inválido ou se estiverem em resoluções distintas.
pub fn h3_grid_distance(origin: u64, destination: u64) -> Result<i32, PortError> {
    let cell_origin = CellIndex::try_from(origin)
        .map_err(|e| PortError::ValidationError(format!("Índice H3 de origem inválido: {e}")))?;
    let cell_dest = CellIndex::try_from(destination)
        .map_err(|e| PortError::ValidationError(format!("Índice H3 de destino inválido: {e}")))?;

    cell_origin.grid_distance(cell_dest).map_err(|e| {
        PortError::ValidationError(format!("Falha no cálculo de distância na malha H3: {e}"))
    })
}

/// Enriquece um `RecordBatch` Apache Arrow anexando uma coluna `UInt64` com os índices espaciais H3.
///
/// Processa fatias contíguas de latitude e longitude com máxima velocidade e sem cópia desnecessária.
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se as colunas de latitude ou longitude não existirem
/// ou não forem do tipo `Float64`.
pub fn append_h3_column(
    batch: &RecordBatch,
    lat_column: &str,
    lng_column: &str,
    target_column: &str,
    resolution: u8,
) -> Result<RecordBatch, PortError> {
    let lat_idx = batch
        .schema()
        .index_of(lat_column)
        .map_err(|_| PortError::ValidationError(format!("Coluna '{lat_column}' não encontrada")))?;

    let lng_idx = batch
        .schema()
        .index_of(lng_column)
        .map_err(|_| PortError::ValidationError(format!("Coluna '{lng_column}' não encontrada")))?;

    let lat_array = batch
        .column(lat_idx)
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| {
            PortError::ValidationError(format!("Coluna '{lat_column}' deve ser do tipo Float64"))
        })?;

    let lng_array = batch
        .column(lng_idx)
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| {
            PortError::ValidationError(format!("Coluna '{lng_column}' deve ser do tipo Float64"))
        })?;

    let num_rows = batch.num_rows();
    let mut builder = UInt64Builder::with_capacity(num_rows);

    for i in 0..num_rows {
        if lat_array.is_null(i) || lng_array.is_null(i) {
            builder.append_null();
            continue;
        }

        let lat = lat_array.value(i);
        let lng = lng_array.value(i);

        match coord_to_h3_index(lat, lng, resolution) {
            Ok(index) => builder.append_value(index),
            Err(_) => builder.append_null(),
        }
    }

    let h3_array: ArrayRef = Arc::new(builder.finish());

    // Constrói novo esquema Arrow incluindo a coluna H3
    let mut fields: Vec<Arc<Field>> = batch.schema().fields().to_vec();
    fields.push(Arc::new(Field::new(target_column, DataType::UInt64, true)));
    let new_schema = Arc::new(Schema::new(fields));

    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns.push(h3_array);

    RecordBatch::try_new(new_schema, columns).map_err(|e| {
        PortError::ValidationError(format!("Falha ao gerar RecordBatch com coluna H3: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::UInt64Array;

    #[test]
    fn test_praça_da_se_h3() {
        let lat = -23.550520;
        let lng = -46.633308;
        let h3_res7 = coord_to_h3_index(lat, lng, 7).unwrap();
        let h3_res8 = coord_to_h3_index(lat, lng, 8).unwrap();
        assert_ne!(h3_res7, 0);
        assert_ne!(h3_res8, 0);

        // Conversão reversa para coordenadas (tolerância submétrica/quilométrica compatível com H3)
        let (lat_rev, lng_rev) = h3_index_to_coord(h3_res8).unwrap();
        assert!((lat - lat_rev).abs() < 0.01);
        assert!((lng - lng_rev).abs() < 0.01);
    }

    #[test]
    fn test_grid_disk_neighbors() {
        let lat = -15.7975;
        let lng = -47.8919; // Brasília
        let center = coord_to_h3_index(lat, lng, 8).unwrap();

        // Raio 1 deve conter exatamente 7 células (o centro + 6 hexágonos contíguos)
        let disk_1 = h3_grid_disk(center, 1).unwrap();
        assert_eq!(disk_1.len(), 7);
        assert!(disk_1.contains(&center));
    }

    #[test]
    fn test_distance_between_adjacent_cells() {
        let lat = -15.7975;
        let lng = -47.8919;
        let center = coord_to_h3_index(lat, lng, 8).unwrap();
        let disk_1 = h3_grid_disk(center, 1).unwrap();
        let neighbor = disk_1.iter().find(|&&c| c != center).copied().unwrap();

        let dist = h3_grid_distance(center, neighbor).unwrap();
        assert_eq!(dist, 1);
    }

    #[test]
    fn test_append_h3_column_to_batch() {
        let lat_array = Arc::new(Float64Array::from(vec![
            Some(-23.5505),
            None,
            Some(-22.9068),
        ]));
        let lng_array = Arc::new(Float64Array::from(vec![
            Some(-46.6333),
            None,
            Some(-43.1729),
        ]));

        let schema = Arc::new(Schema::new(vec![
            Field::new("lat", DataType::Float64, true),
            Field::new("lng", DataType::Float64, true),
        ]));

        let batch = RecordBatch::try_new(schema, vec![lat_array, lng_array]).unwrap();
        let enriched = append_h3_column(&batch, "lat", "lng", "h3_res8", 8).unwrap();

        assert_eq!(enriched.num_columns(), 3);
        assert_eq!(enriched.num_rows(), 3);

        let h3_col = enriched
            .column(2)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();

        assert!(h3_col.is_valid(0));
        assert!(h3_col.is_null(1)); // Linha com nulo deve produzir H3 nulo
        assert!(h3_col.is_valid(2));
    }
}
