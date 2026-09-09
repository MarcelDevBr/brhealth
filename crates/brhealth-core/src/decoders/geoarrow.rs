// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Decodificador e construtor de geometrias vetoriais no formato GeoArrow.
//!
//! Implementa a especificação GeoArrow para pontos bidimensionais $(x, y) = (\text{lon}, \text{lat})$
//! em memória contígua alinhada para processamento vetorizado SIMD e intercâmbio Zero-Copy.

use std::sync::Arc;

use arrow::array::{Array, Float64Array, StructArray};
use arrow::datatypes::{DataType, Field, Fields, Schema};
use arrow::record_batch::RecordBatch;

use crate::domain::ports::outbound::PortError;

/// Construtor de colunas geométricas GeoArrow do tipo `geoarrow.point`.
#[derive(Debug, Default, Clone)]
pub struct GeoArrowDecoder;

impl GeoArrowDecoder {
    /// Cria uma nova instância do decodificador GeoArrow.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Cria um campo Arrow com metadados da extensão GeoArrow para pontos WGS84 (EPSG:4326).
    #[must_use]
    pub fn point_field(name: &str) -> Arc<Field> {
        let x_field = Arc::new(Field::new("x", DataType::Float64, false));
        let y_field = Arc::new(Field::new("y", DataType::Float64, false));
        let fields = Fields::from(vec![x_field, y_field]);

        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "ARROW:extension:name".to_string(),
            "geoarrow.point".to_string(),
        );
        metadata.insert(
            "ARROW:extension:metadata".to_string(),
            r#"{"crs":"OGC:CRS84"}"#.to_string(),
        );

        Arc::new(Field::new(name, DataType::Struct(fields), true).with_metadata(metadata))
    }

    /// Constrói um `StructArray` de pontos GeoArrow a partir de vetores contíguos de coordenadas.
    pub fn build_points(
        &self,
        longitudes: &[f64],
        latitudes: &[f64],
    ) -> Result<Arc<StructArray>, PortError> {
        if longitudes.len() != latitudes.len() {
            return Err(PortError::ValidationError(format!(
                "Dimensões incompatíveis: {} longitudes vs {} latitudes",
                longitudes.len(),
                latitudes.len()
            )));
        }

        let x_array = Arc::new(Float64Array::from(longitudes.to_vec()));
        let y_array = Arc::new(Float64Array::from(latitudes.to_vec()));

        let x_field = Arc::new(Field::new("x", DataType::Float64, false));
        let y_field = Arc::new(Field::new("y", DataType::Float64, false));
        let fields = Fields::from(vec![x_field, y_field]);

        let arrays: Vec<Arc<dyn Array>> = vec![x_array, y_array];
        let struct_array = StructArray::try_new(fields, arrays, None).map_err(|e| {
            PortError::TransformationError(format!("Falha ao montar GeoArrow: {e}"))
        })?;

        Ok(Arc::new(struct_array))
    }

    /// Anexa uma coluna de geometria GeoArrow a um `RecordBatch` existente usando as colunas de latitude e longitude.
    pub fn append_geometry_column(
        &self,
        batch: &RecordBatch,
        lon_col: &str,
        lat_col: &str,
        geom_col_name: &str,
    ) -> Result<RecordBatch, PortError> {
        let lon_array = batch
            .column_by_name(lon_col)
            .ok_or_else(|| {
                PortError::ValidationError(format!("Coluna '{lon_col}' não encontrada"))
            })?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| {
                PortError::ValidationError(format!("Coluna '{lon_col}' não é Float64"))
            })?;

        let lat_array = batch
            .column_by_name(lat_col)
            .ok_or_else(|| {
                PortError::ValidationError(format!("Coluna '{lat_col}' não encontrada"))
            })?
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| {
                PortError::ValidationError(format!("Coluna '{lat_col}' não é Float64"))
            })?;

        let num_rows = batch.num_rows();
        let mut lons = Vec::with_capacity(num_rows);
        let mut lats = Vec::with_capacity(num_rows);

        for i in 0..num_rows {
            lons.push(lon_array.value(i));
            lats.push(lat_array.value(i));
        }

        let geom_struct = self.build_points(&lons, &lats)?;

        let mut fields = batch.schema().fields().to_vec();
        fields.push(Self::point_field(geom_col_name));

        let mut cols = batch.columns().to_vec();
        cols.push(geom_struct);

        RecordBatch::try_new(Arc::new(Schema::new(fields)), cols)
            .map_err(|e| PortError::TransformationError(format!("Falha ao estender schema: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geoarrow_build_points() {
        let decoder = GeoArrowDecoder::new();
        let lons = vec![-46.6333, -43.1729];
        let lats = vec![-23.5505, -22.9068];

        let points = decoder.build_points(&lons, &lats).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points.num_columns(), 2);
    }
}
