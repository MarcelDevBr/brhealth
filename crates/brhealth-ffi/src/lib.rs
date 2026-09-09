// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! C-ABI plana e Arrow C Data Interface para o motor analítico BRHealth.
//!
//! Fornece bindings estáveis C-ABI para consumo direto por bibliotecas compartilhadas
//! em C, C++20, Python (via `ctypes` ou `cffi`) e Java 21+ (Foreign Function & Memory API).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::OnceLock;

use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::record_batch::RecordBatch;

use brhealth_core::domain::analytics::csap::{classify_cid10, compute_primary_care_roi};
use brhealth_core::domain::spatial::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::calculate_ibge_dv;
use brhealth_core::ffi::export_record_batch_to_c;

/// Código de sucesso retornado pelas funções FFI.
pub const BRHEALTH_SUCCESS: i32 = 0;
/// Ponteiro nulo inválido fornecido como argumento.
pub const BRHEALTH_ERR_NULL_PTR: i32 = -1;
/// Argumento inválido ou fora dos limites estabelecidos.
pub const BRHEALTH_ERR_INVALID_ARG: i32 = -2;
/// Erro de computação ou transformação analítica.
pub const BRHEALTH_ERR_TRANSFORM_FAILED: i32 = -3;

static VERSION_C_STR: OnceLock<CString> = OnceLock::new();

/// Retorna a versão canônica da biblioteca BRHealth em string C (terminada em null).
#[unsafe(no_mangle)]
pub extern "C" fn brhealth_version() -> *const c_char {
    VERSION_C_STR
        .get_or_init(|| {
            CString::new(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| CString::default())
        })
        .as_ptr()
}

/// Calcula o Dígito Verificador (DV) do IBGE (Luhn Módulo 10) para um código de 6 dígitos.
///
/// # Safety
///
/// Requer ponteiro válido e terminado em null para `ibge_6digits` e ponteiro válido para `out_dv`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_calculate_ibge_dv(
    ibge_6digits: *const c_char,
    out_dv: *mut u8,
) -> i32 {
    if ibge_6digits.is_null() || out_dv.is_null() {
        return BRHEALTH_ERR_NULL_PTR;
    }

    let c_str = unsafe { CStr::from_ptr(ibge_6digits) };
    let code_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return BRHEALTH_ERR_INVALID_ARG,
    };

    match calculate_ibge_dv(code_str) {
        Ok(dv) => {
            unsafe { *out_dv = dv };
            BRHEALTH_SUCCESS
        }
        Err(_) => BRHEALTH_ERR_INVALID_ARG,
    }
}

/// Converte latitude e longitude em um índice hexagonal Uber H3 de 64 bits.
///
/// # Safety
///
/// Requer ponteiro válido não-nulo para `out_h3`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_coord_to_h3(
    lat: f64,
    lon: f64,
    resolution: u8,
    out_h3: *mut u64,
) -> i32 {
    if out_h3.is_null() {
        return BRHEALTH_ERR_NULL_PTR;
    }

    match coord_to_h3_index(lat, lon, resolution) {
        Ok(h3_index) => {
            unsafe { *out_h3 = h3_index };
            BRHEALTH_SUCCESS
        }
        Err(_) => BRHEALTH_ERR_INVALID_ARG,
    }
}

/// Classifica um código CID-10 conforme os 19 grupos de CSAP (Portaria MS/SAS nº 221/2008).
///
/// Grava o identificador numérico do grupo (1 a 19) em `out_group_id`, ou 0 caso não seja CSAP.
///
/// # Safety
///
/// Requer ponteiro válido terminado em null para `cid10` e ponteiro válido para `out_group_id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_classify_csap(
    cid10: *const c_char,
    out_group_id: *mut u8,
) -> i32 {
    if cid10.is_null() || out_group_id.is_null() {
        return BRHEALTH_ERR_NULL_PTR;
    }

    let c_str = unsafe { CStr::from_ptr(cid10) };
    let cid_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return BRHEALTH_ERR_INVALID_ARG,
    };

    match classify_cid10(cid_str) {
        Some(group) => {
            unsafe { *out_group_id = group.id() };
            BRHEALTH_SUCCESS
        }
        None => {
            unsafe { *out_group_id = 0 };
            BRHEALTH_SUCCESS
        }
    }
}

/// Calcula o Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária.
///
/// # Safety
///
/// Requer ponteiro válido para `out_roi`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_compute_primary_care_roi(
    avoidable_cost: f64,
    investment: f64,
    attributable_fraction: f64,
    out_roi: *mut f64,
) -> i32 {
    if out_roi.is_null() {
        return BRHEALTH_ERR_NULL_PTR;
    }

    match compute_primary_care_roi(avoidable_cost, investment, attributable_fraction) {
        Ok(roi) => {
            unsafe { *out_roi = roi };
            BRHEALTH_SUCCESS
        }
        Err(_) => BRHEALTH_ERR_INVALID_ARG,
    }
}

/// Exporta um lote tabular `RecordBatch` Apache Arrow para a Arrow C Data Interface.
///
/// # Safety
///
/// Requer ponteiros válidos e alocados para `batch_ptr`, `out_array` e `out_schema`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_export_arrow_batch(
    batch_ptr: *const RecordBatch,
    out_array: *mut FFI_ArrowArray,
    out_schema: *mut FFI_ArrowSchema,
) -> i32 {
    if batch_ptr.is_null() || out_array.is_null() || out_schema.is_null() {
        return BRHEALTH_ERR_NULL_PTR;
    }

    let batch = unsafe { &*batch_ptr };
    match unsafe { export_record_batch_to_c(batch, out_array, out_schema) } {
        Ok(()) => BRHEALTH_SUCCESS,
        Err(_) => BRHEALTH_ERR_TRANSFORM_FAILED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_version_c_str() {
        let ptr = brhealth_version();
        assert!(!ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ptr) };
        assert_eq!(c_str.to_str().unwrap(), "1.0.0");
    }

    #[test]
    fn test_ibge_dv_c_abi() {
        let code = CString::new("355030").unwrap();
        let mut dv = 0u8;
        let status = unsafe { brhealth_calculate_ibge_dv(code.as_ptr(), &mut dv as *mut _) };
        assert_eq!(status, BRHEALTH_SUCCESS);
        assert_eq!(dv, 8);

        // Testar ponteiro nulo
        let status_null =
            unsafe { brhealth_calculate_ibge_dv(std::ptr::null(), &mut dv as *mut _) };
        assert_eq!(status_null, BRHEALTH_ERR_NULL_PTR);
    }

    #[test]
    fn test_h3_c_abi() {
        let mut h3 = 0u64;
        let status = unsafe { brhealth_coord_to_h3(-23.55052, -46.633308, 8, &mut h3 as *mut _) };
        assert_eq!(status, BRHEALTH_SUCCESS);
        assert_eq!(h3, 615446139314896895u64);
    }

    #[test]
    fn test_csap_c_abi() {
        let mut group = 0u8;
        let cid_csap = CString::new("J450").unwrap();
        let status = unsafe { brhealth_classify_csap(cid_csap.as_ptr(), &mut group as *mut _) };
        assert_eq!(status, BRHEALTH_SUCCESS);
        assert_eq!(group, 7); // Grupo 7 = Asma

        let cid_non_csap = CString::new("S060").unwrap();
        let status2 =
            unsafe { brhealth_classify_csap(cid_non_csap.as_ptr(), &mut group as *mut _) };
        assert_eq!(status2, BRHEALTH_SUCCESS);
        assert_eq!(group, 0); // Não-CSAP
    }

    #[test]
    fn test_roi_c_abi() {
        let mut roi = 0.0f64;
        let status = unsafe {
            brhealth_compute_primary_care_roi(500_000.0, 100_000.0, 0.50, &mut roi as *mut _)
        };
        assert_eq!(status, BRHEALTH_SUCCESS);
        assert!((roi - 1.5).abs() < 1e-6);
    }
}
