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
