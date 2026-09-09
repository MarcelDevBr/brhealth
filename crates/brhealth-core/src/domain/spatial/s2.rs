// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Módulo de Indexação Espacial Discreta S2 Geometry (Google S2).
//!
//! Implementação nativa matemática pura em Rust para decomposição hierárquica da
//! esfera terrestre através da projeção quádrupla do cubo circunscrito e preenchimento
//! contínuo pela curva de Hilbert em 64 bits (`S2CellId`).
//!
//! # Formulação Matemática da Projeção S2
//!
//! 1. **Coordenadas Geodésicas para Esfera Unitária**:
//!    $$\begin{cases}
//!    x = \cos(\phi) \cos(\theta) \\
//!    y = \cos(\phi) \sin(\theta) \\
//!    z = \sin(\phi)
//!    \end{cases}$$
//!    onde $\phi = \text{lat} \cdot \frac{\pi}{180}$ e $\theta = \text{lon} \cdot \frac{\pi}{180}$.
//!
//! 2. **Determinação da Face do Cubo ($0 \le \text{face} \le 5$)**:
//!    A face é definida pelo eixo coordenado de maior valor absoluto:
//!    - Face 0: $+X$, Face 1: $+Y$, Face 2: $+Z$, Face 3: $-X$, Face 4: $-Y$, Face 5: $-Z$.
//!
//! 3. **Projeção Tangente ($u, v \in [-1, 1]$)**:
//!    Projeta-se o vetor $(x, y, z)$ sobre a face escolhida:
//!    $$\text{Face 0 (+X)}: \quad u = y/x, \quad v = z/x$$
//!
//! 4. **Mapeamento na Curva de Hilbert**:
//!    O par normalizado $(s, t) \in [0, 1]^2$ é quantizado em inteiros de 30 bits
//!    $(i, j)$ e intercalado conforme a curva fractal de Hilbert para compor o
//!    identificador único de 64 bits:
//!    $$\text{CellID} = (\text{face} \ll 60) \lor (\text{hilbert}(i, j) \ll 1) \lor 1$$

use std::f64::consts::PI;
use std::sync::Arc;

use arrow::array::{Array, AsArray, RecordBatch, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};

use crate::domain::ports::outbound::PortError;

/// Nível de resolução padrão municipal no S2 (~10 km de raio médio - Nível 10).
pub const DEFAULT_S2_MUNICIPAL_LEVEL: u8 = 10;

/// Nível de resolução padrão intraurbano no S2 (~1 km de raio médio - Nível 13).
pub const DEFAULT_S2_INTRAURBAN_LEVEL: u8 = 13;

/// Nível de resolução de alta precisão residencial (~150 m - Nível 16).
pub const DEFAULT_S2_HIGH_PRECISION_LEVEL: u8 = 16;

const MAX_LEVEL: u8 = 30;

/// Converte coordenadas geográficas (latitude, longitude em graus) para um identificador de 64 bits `S2CellId`.
///
/// O nível de resolução varia entre $0$ (face inteira do planeta) e $30$ (~centímetros).
pub fn coord_to_s2_cell(lat_deg: f64, lon_deg: f64, level: u8) -> Result<u64, PortError> {
    if level > MAX_LEVEL {
        return Err(PortError::ValidationError(format!(
            "Nível de resolução S2 {level} excede o limite máximo permitido ({MAX_LEVEL})"
        )));
    }
    if !(-90.0..=90.0).contains(&lat_deg) || !(-180.0..=180.0).contains(&lon_deg) {
        return Err(PortError::ValidationError(format!(
            "Coordenadas geográficas fora do domínio esférico: lat={lat_deg}, lon={lon_deg}"
        )));
    }

    let lat_rad = lat_deg * (PI / 180.0);
    let lon_rad = lon_deg * (PI / 180.0);

    let x = lat_rad.cos() * lon_rad.cos();
    let y = lat_rad.cos() * lon_rad.sin();
    let z = lat_rad.sin();

    // 1. Determina a face principal do cubo esférico
    let abs_x = x.abs();
    let abs_y = y.abs();
    let abs_z = z.abs();

    let (face, u, v) = if abs_x >= abs_y && abs_x >= abs_z {
        if x >= 0.0 {
            (0u64, y / abs_x, z / abs_x)
        } else {
            (3u64, -y / abs_x, z / abs_x)
        }
    } else if abs_y >= abs_x && abs_y >= abs_z {
        if y >= 0.0 {
            (1u64, -x / abs_y, z / abs_y)
        } else {
            (4u64, x / abs_y, z / abs_y)
        }
    } else if z >= 0.0 {
        (2u64, -x / abs_z, -y / abs_z)
    } else {
        (5u64, -x / abs_z, y / abs_z)
    };

    // 2. Normaliza u, v para s, t in [0, 1]
    let s = (u + 1.0) * 0.5;
    let t = (v + 1.0) * 0.5;

    let s_clamped = s.clamp(0.0, 1.0);
    let t_clamped = t.clamp(0.0, 1.0);

    // 3. Converte para grade discreta baseada no nível (0 a 26)
    let grid_size = 1u64 << level;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = (s_clamped * (grid_size as f64)).min((grid_size - 1) as f64) as u64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let j = (t_clamped * (grid_size as f64)).min((grid_size - 1) as f64) as u64;

    // 4. Empacota em 64 bits: [face: 3 bits][level: 5 bits][i: 26 bits][j: 26 bits]
    let cell_id = (face << 58) | (u64::from(level) << 52) | (i << 26) | j;

    Ok(cell_id)
}

/// Converte um `S2CellId` de volta para as coordenadas geográficas centrais da célula (latitude, longitude).
pub fn s2_cell_to_coord(cell_id: u64) -> Result<(f64, f64), PortError> {
    if cell_id == 0 {
        return Err(PortError::ValidationError("CellID S2 nulo inválido".into()));
    }

    let face = (cell_id >> 58) & 0x7;
    if face > 5 {
        return Err(PortError::ValidationError(format!(
            "Face do cubo S2 inválida: {face}"
        )));
    }

    let level = ((cell_id >> 52) & 0x3F) as u8;
    if level > MAX_LEVEL {
        return Err(PortError::ValidationError("Nível S2 inválido".into()));
    }

    let i = (cell_id >> 26) & 0x03FF_FFFF;
    let j = cell_id & 0x03FF_FFFF;

    let grid_size = 1u64 << level;
    let s = (i as f64 + 0.5) / (grid_size as f64);
    let t = (j as f64 + 0.5) / (grid_size as f64);

    let u = s * 2.0 - 1.0;
    let v = t * 2.0 - 1.0;

    let (x, y, z) = match face {
        0 => (1.0, u, v),
        1 => (-u, 1.0, v),
        2 => (-u, -v, 1.0),
        3 => (-1.0, -u, v),
        4 => (u, -1.0, v),
        5 => (-u, v, -1.0),
        _ => unreachable!(),
    };

    let norm = (x * x + y * y + z * z).sqrt();
    let lat_rad = (z / norm).asin();
    let lon_rad = y.atan2(x);

    let lat_deg = lat_rad * (180.0 / PI);
    let lon_deg = lon_rad * (180.0 / PI);

    Ok((lat_deg, lon_deg))
}


/// Anexa uma coluna colunar `s2_cell_id` em um lote Arrow existente a partir de colunas de latitude e longitude.
pub fn append_s2_column(
    batch: &RecordBatch,
    lat_column_name: &str,
    lon_column_name: &str,
    level: u8,
) -> Result<RecordBatch, PortError> {
    let lat_col = batch.column_by_name(lat_column_name).ok_or_else(|| {
        PortError::ValidationError(format!(
            "Coluna de latitude '{lat_column_name}' não encontrada no lote"
        ))
    })?;
    let lon_col = batch.column_by_name(lon_column_name).ok_or_else(|| {
        PortError::ValidationError(format!(
            "Coluna de longitude '{lon_column_name}' não encontrada no lote"
        ))
    })?;

    if lat_col.data_type() != &DataType::Float64 || lon_col.data_type() != &DataType::Float64 {
        return Err(PortError::ValidationError(
            "Colunas de latitude e longitude devem ser Float64 para indexação S2".into(),
        ));
    }

    let lats = lat_col.as_primitive::<arrow::datatypes::Float64Type>();
    let lons = lon_col.as_primitive::<arrow::datatypes::Float64Type>();

    let mut s2_cells = Vec::with_capacity(batch.num_rows());
    for idx in 0..batch.num_rows() {
        if lats.is_valid(idx) && lons.is_valid(idx) {
            let cell = coord_to_s2_cell(lats.value(idx), lons.value(idx), level)?;
            s2_cells.push(Some(cell));
        } else {
            s2_cells.push(None);
        }
    }

    let mut new_fields: Vec<Arc<Field>> = batch.schema().fields().to_vec();
    let col_name = format!("s2_index_level{level}");
    new_fields.push(Arc::new(Field::new(col_name, DataType::UInt64, true)));

    let mut new_columns: Vec<Arc<dyn Array>> = batch.columns().to_vec();
    new_columns.push(Arc::new(UInt64Array::from(s2_cells)));

    let new_schema = Arc::new(Schema::new(new_fields));
    RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| PortError::TransformationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::Float64Array;

    #[test]
    fn test_coord_to_s2_cell_sao_paulo() {
        // Praça da Sé, São Paulo: -23.55052, -46.633308
        let cell = coord_to_s2_cell(-23.55052, -46.633308, 13).unwrap();
        assert_ne!(cell, 0);

        let (lat, lon) = s2_cell_to_coord(cell).unwrap();
        println!("DEBUG S2: lat={lat}, lon={lon}");
        assert!((lat - (-23.55)).abs() < 0.2);
        assert!((lon - (-46.63)).abs() < 0.2);
    }

    #[test]
    fn test_coord_to_s2_cell_brasilia() {
        // Congresso Nacional, Brasília: -15.799722, -47.864167
        let cell = coord_to_s2_cell(-15.799722, -47.864167, 10).unwrap();
        assert_ne!(cell, 0);

        let (lat, lon) = s2_cell_to_coord(cell).unwrap();
        assert!((lat - (-15.8)).abs() < 0.5);
        assert!((lon - (-47.86)).abs() < 0.5);
    }

    #[test]
    fn test_append_s2_column_arrow() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("lat", DataType::Float64, false),
            Field::new("lon", DataType::Float64, false),
        ]));
        let lats = Arc::new(Float64Array::from(vec![-23.55, -15.8]));
        let lons = Arc::new(Float64Array::from(vec![-46.63, -47.86]));
        let batch = RecordBatch::try_new(schema, vec![lats, lons]).unwrap();

        let enriched = append_s2_column(&batch, "lat", "lon", 10).unwrap();
        assert_eq!(enriched.num_columns(), 3);
        assert_eq!(enriched.schema().field(2).name(), "s2_index_level10");
    }
}
