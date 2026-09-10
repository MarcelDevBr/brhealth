// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Funções utilitárias seguras para extração colunar dos microdados DATASUS.

use arrow::array::{Array, Date32Array, Float64Array, StringArray};
use arrow::record_batch::RecordBatch;

/// Extrai com segurança uma fatia de string de uma coluna do `RecordBatch`.
#[inline]
pub fn get_str_value<'a>(batch: &'a RecordBatch, col_name: &str, row: usize) -> Option<&'a str> {
    let idx = batch.schema().index_of(col_name).ok()?;
    let col = batch.column(idx).as_any().downcast_ref::<StringArray>()?;
    if col.is_valid(row) {
        Some(col.value(row).trim())
    } else {
        None
    }
}

/// Extrai com segurança o valor `Date32` de uma coluna do `RecordBatch`.
#[inline]
pub fn get_date32_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<i32> {
    let idx = batch.schema().index_of(col_name).ok()?;
    let col = batch.column(idx).as_any().downcast_ref::<Date32Array>()?;
    if col.is_valid(row) {
        Some(col.value(row))
    } else {
        None
    }
}

/// Extrai com segurança um valor numérico de ponto flutuante `f64` do `RecordBatch`.
#[inline]
pub fn get_float64_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<f64> {
    let idx = batch.schema().index_of(col_name).ok()?;
    let col = batch.column(idx);
    if let Some(flt_col) = col.as_any().downcast_ref::<Float64Array>() {
        flt_col.is_valid(row).then(|| flt_col.value(row))
    } else if let Some(str_col) = col.as_any().downcast_ref::<StringArray>() {
        str_col
            .is_valid(row)
            .then(|| str_col.value(row).trim().parse::<f64>().ok())
            .flatten()
    } else {
        None
    }
}

/// Extrai com segurança um valor `f32` de uma coluna do `RecordBatch`.
#[inline]
pub fn get_float32_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<f32> {
    get_float64_value(batch, col_name, row).map(|v| v as f32)
}

/// Extrai com segurança um valor `u32` de uma coluna do `RecordBatch`.
#[inline]
pub fn get_u32_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<u32> {
    let idx = batch.schema().index_of(col_name).ok()?;
    let col = batch.column(idx);
    if let Some(str_col) = col.as_any().downcast_ref::<StringArray>() {
        str_col
            .is_valid(row)
            .then(|| str_col.value(row).trim().parse::<u32>().ok())
            .flatten()
    } else {
        None
    }
}

/// Extrai com segurança um valor `u16` de uma coluna do `RecordBatch`.
#[inline]
pub fn get_u16_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<u16> {
    get_u32_value(batch, col_name, row).and_then(|v| u16::try_from(v).ok())
}

/// Extrai com segurança um valor `u8` de uma coluna do `RecordBatch`.
#[inline]
pub fn get_u8_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<u8> {
    get_u32_value(batch, col_name, row).and_then(|v| u8::try_from(v).ok())
}

/// Extrai com segurança um valor booleano de uma coluna do `RecordBatch`.
#[inline]
pub fn get_bool_value(batch: &RecordBatch, col_name: &str, row: usize) -> Option<bool> {
    let idx = batch.schema().index_of(col_name).ok()?;
    let col = batch.column(idx);
    if let Some(str_col) = col.as_any().downcast_ref::<StringArray>() {
        if str_col.is_valid(row) {
            let s = str_col.value(row).trim().to_uppercase();
            match s.as_str() {
                "1" | "S" | "SIM" | "T" | "TRUE" => Some(true),
                "0" | "N" | "NAO" | "NÃO" | "F" | "FALSE" => Some(false),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

use std::sync::Arc;
use arrow::array::{
    new_null_array, ArrayRef, Date32Builder, Float64Builder, StringBuilder,
    UInt16Builder, UInt32Builder, UInt8Builder,
};
use arrow::datatypes::DataType;
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Constrói um array de nulos de alta performance $O(1)$ para um tipo de dado.
#[inline]
pub fn build_null_col(data_type: &DataType, len: usize) -> ArrayRef {
    new_null_array(data_type, len)
}

/// Constrói uma coluna com valor constante de string sem alocações repetidas.
#[inline]
pub fn build_constant_str_col(val: &str, len: usize) -> ArrayRef {
    Arc::new(StringArray::from_iter_values(std::iter::repeat_n(val, len)))
}

/// Constrói a coluna canônica de identificador de registro (`record_id`).
pub fn build_record_id_col(
    batch: &RecordBatch,
    col_name: &str,
    prefix: &str,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
    for i in 0..num_rows {
        let id = get_str_value(batch, col_name, i).unwrap_or("");
        if id.is_empty() {
            builder.append_value(format!("{prefix}_{i}"));
        } else {
            builder.append_value(id);
        }
    }
    Arc::new(builder.finish())
}

/// Constrói a coluna canônica de município harmonizado para 7 dígitos do IBGE.
pub fn build_harmonized_ibge_col(
    batch: &RecordBatch,
    col_name: &str,
    default_mun: &str,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
    for i in 0..num_rows {
        let resolved = get_str_value(batch, col_name, i)
            .and_then(|m| harmonize_ibge_code(m).ok())
            .unwrap_or_else(|| default_mun.to_string());
        builder.append_value(resolved);
    }
    Arc::new(builder.finish())
}

/// Constrói a coluna canônica de sexo biológico ("M", "F", "U").
pub fn build_sex_col(batch: &RecordBatch, col_name: &str, num_rows: usize) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 2);
    for i in 0..num_rows {
        let s = match get_str_value(batch, col_name, i) {
            Some("1" | "M") => "M",
            Some("2" | "F") => "F",
            _ => "U",
        };
        builder.append_value(s);
    }
    Arc::new(builder.finish())
}

/// Constrói a coluna canônica de raça/etnia conforme categorização do DATASUS/IBGE.
pub fn build_race_col(batch: &RecordBatch, col_name: &str, num_rows: usize) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
    for i in 0..num_rows {
        let r = match get_str_value(batch, col_name, i) {
            Some("1") => Some("Branca"),
            Some("2") => Some("Preta"),
            Some("3") => Some("Amarela"),
            Some("4") => Some("Parda"),
            Some("5") => Some("Indígena"),
            _ => None,
        };
        if let Some(race) = r {
            builder.append_value(race);
        } else {
            builder.append_null();
        }
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `Date32` com valor padrão caso nulo.
pub fn build_date32_col(
    batch: &RecordBatch,
    col_name: &str,
    default_val: i32,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = Date32Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        let d = get_date32_value(batch, col_name, i).unwrap_or(default_val);
        builder.append_value(d);
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `Date32` opcional (nulo se ausente).
pub fn build_date32_opt_col(batch: &RecordBatch, col_name: &str, num_rows: usize) -> ArrayRef {
    let mut builder = Date32Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        if let Some(d) = get_date32_value(batch, col_name, i) {
            builder.append_value(d);
        } else {
            builder.append_null();
        }
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `Utf8` com valor padrão caso nulo.
pub fn build_str_col(
    batch: &RecordBatch,
    col_name: &str,
    default_val: &str,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * default_val.len().max(6));
    for i in 0..num_rows {
        let s = get_str_value(batch, col_name, i).unwrap_or(default_val);
        builder.append_value(s);
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `Utf8` opcional (nulo se ausente).
pub fn build_str_opt_col(
    batch: &RecordBatch,
    col_name: &str,
    avg_len: usize,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = StringBuilder::with_capacity(num_rows, num_rows * avg_len);
    for i in 0..num_rows {
        if let Some(s) = get_str_value(batch, col_name, i) {
            builder.append_value(s);
        } else {
            builder.append_null();
        }
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `UInt8` opcional (nulo se ausente).
pub fn build_u8_opt_col(batch: &RecordBatch, col_name: &str, num_rows: usize) -> ArrayRef {
    let mut builder = UInt8Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        if let Some(v) = get_u8_value(batch, col_name, i) {
            builder.append_value(v);
        } else {
            builder.append_null();
        }
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `UInt16` com valor padrão caso nulo.
pub fn build_u16_col(
    batch: &RecordBatch,
    col_name: &str,
    default_val: u16,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = UInt16Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        let v = get_u16_value(batch, col_name, i).unwrap_or(default_val);
        builder.append_value(v);
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `UInt16` opcional (nulo se ausente).
pub fn build_u16_opt_col(batch: &RecordBatch, col_name: &str, num_rows: usize) -> ArrayRef {
    let mut builder = UInt16Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        if let Some(v) = get_u16_value(batch, col_name, i) {
            builder.append_value(v);
        } else {
            builder.append_null();
        }
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `UInt32` com valor padrão caso nulo.
pub fn build_u32_col(
    batch: &RecordBatch,
    col_name: &str,
    default_val: u32,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = UInt32Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        let v = get_u32_value(batch, col_name, i).unwrap_or(default_val);
        builder.append_value(v);
    }
    Arc::new(builder.finish())
}

/// Constrói coluna `Float64` com valor padrão caso nulo.
pub fn build_f64_col(
    batch: &RecordBatch,
    col_name: &str,
    default_val: f64,
    num_rows: usize,
) -> ArrayRef {
    let mut builder = Float64Builder::with_capacity(num_rows);
    for i in 0..num_rows {
        let v = get_float64_value(batch, col_name, i).unwrap_or(default_val);
        builder.append_value(v);
    }
    Arc::new(builder.finish())
}
