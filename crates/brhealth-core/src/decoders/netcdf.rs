// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Decodificador colunar para matrizes e grades climáticas (NetCDF / ERA5).
//!
//! Converte dimensões de grade geodésica $(\text{lat}, \text{lon}, \text{tempo})$
//! e variáveis meteorológicas escalares ($T_{2m}$, precipitação, umidade)
//! em estruturas tabulares contíguas `RecordBatch` alinhadas a 64 bytes.

use std::sync::Arc;

use arrow::array::{Float32Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::domain::ports::outbound::PortError;

/// Representação de uma variável em grade multidimensional desdobrada.
#[derive(Debug, Clone)]
pub struct ClimateGridVariable {
    /// Nome da variável meteorológica (ex: `"temperature_2m"`).
    pub name: String,
    /// Unidade de medida (ex: `"Kelvin"`, `"mm"`).
    pub unit: String,
    /// Valores escalares achatados correspondentes às coordenadas $(\text{lat}, \text{lon})$.
    pub values: Vec<f32>,
}

/// Decodificador colunar de matrizes climáticas para Apache Arrow.
#[derive(Debug, Default, Clone)]
pub struct NetCDFGridDecoder;

impl NetCDFGridDecoder {
    /// Cria uma nova instância do decodificador de grades.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Decodifica e achata uma matriz de grade climática em um `RecordBatch` colunar.
    ///
    /// # Formulação
    ///
    /// Dado o produto cartesiano de latitudes $\Lambda$ e longitudes $\Phi$, cada ponto
    /// amostral $(\lambda_i, \phi_j)$ é pareado com o valor pontual da variável meteorológica $V_{ij}$.
    pub fn decode_grid_to_batch(
        &self,
        latitudes: &[f64],
        longitudes: &[f64],
        timestamp_iso: &str,
        variables: &[ClimateGridVariable],
    ) -> Result<RecordBatch, PortError> {
        let total_cells = latitudes.len() * longitudes.len();

        let mut lat_col = Vec::with_capacity(total_cells);
        let mut lon_col = Vec::with_capacity(total_cells);
        let mut time_col = Vec::with_capacity(total_cells);

        for &lat in latitudes {
            for &lon in longitudes {
                lat_col.push(lat);
                lon_col.push(lon);
                time_col.push(timestamp_iso);
            }
        }

        let mut fields = vec![
            Arc::new(Field::new("latitude", DataType::Float64, false)),
            Arc::new(Field::new("longitude", DataType::Float64, false)),
            Arc::new(Field::new("timestamp", DataType::Utf8, false)),
        ];

        let mut columns: Vec<Arc<dyn arrow::array::Array>> = vec![
            Arc::new(Float64Array::from(lat_col)),
            Arc::new(Float64Array::from(lon_col)),
            Arc::new(StringArray::from(time_col)),
        ];

        for var in variables {
            if var.values.len() != total_cells {
                return Err(PortError::ValidationError(format!(
                    "Variável '{}' possui {} elementos, esperado {} células da grade",
                    var.name,
                    var.values.len(),
                    total_cells
                )));
            }

            fields.push(Arc::new(Field::new(&var.name, DataType::Float32, true)));
            columns.push(Arc::new(Float32Array::from(var.values.clone())));
        }

        let schema = Arc::new(Schema::new(fields));
        RecordBatch::try_new(schema, columns)
            .map_err(|e| PortError::TransformationError(format!("Falha ao montar batch NetCDF: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netcdf_grid_flattening() {
        let decoder = NetCDFGridDecoder::new();
        let lats = vec![-23.5, -23.0];
        let lons = vec![-46.5, -46.0];
        let var = ClimateGridVariable {
            name: "temperature_k".to_string(),
            unit: "K".to_string(),
            values: vec![298.15, 299.10, 297.80, 298.50],
        };

        let batch = decoder
            .decode_grid_to_batch(&lats, &lons, "2024-01-01T12:00:00Z", &[var])
            .unwrap();

        assert_eq!(batch.num_rows(), 4);
        assert_eq!(batch.num_columns(), 4);
    }
}
