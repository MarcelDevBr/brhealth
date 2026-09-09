// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Bindings para Java 21+ e Kotlin via Project Panama Foreign Function & Memory (FFM) API.
//!
//! Fornece pontos de entrada binários C-ABI otimizados para invocação direta pelo
//! `java.lang.foreign.Linker` sem o overhead histórico do JNI tradicional.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use brhealth_core::domain::analytics::csap::classify_cid10;
use brhealth_core::domain::spatial::h3::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::{calculate_ibge_dv, harmonize_ibge_code};

/// Inicializa o subsistema e retorna o código de status (0 = Sucesso).
#[unsafe(no_mangle)]
pub extern "C" fn brhealth_panama_init() -> c_int {
    0
}

/// Harmoniza um código municipal do IBGE para a representação canônica de 7 dígitos.
///
/// # Safety
///
/// Os ponteiros `raw_code_ptr` e `out_buf` devem ser válidos e não-nulos. `out_len` deve ser suficiente.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_panama_harmonize_ibge(
    raw_code_ptr: *const c_char,
    out_buf: *mut c_char,
    out_len: usize,
) -> c_int {
    if raw_code_ptr.is_null() || out_buf.is_null() || out_len < 8 {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(raw_code_ptr) };
    let raw_code = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    match harmonize_ibge_code(raw_code) {
        Ok(harmonized) => {
            let c_res = match CString::new(harmonized) {
                Ok(c) => c,
                Err(_) => return -3,
            };
            let bytes = c_res.as_bytes_with_nul();
            if bytes.len() > out_len {
                return -4;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), out_buf, bytes.len());
            }
            0
        }
        Err(_) => -5,
    }
}

/// Calcula o Dígito Verificador (Módulo 10 Luhn) do IBGE para um código de 6 dígitos.
///
/// # Safety
///
/// O ponteiro `code_6_digits` deve ser válido e terminado em null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_panama_calculate_dv(code_6_digits: *const c_char) -> c_int {
    if code_6_digits.is_null() {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(code_6_digits) };
    let code = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    match calculate_ibge_dv(code) {
        Ok(dv) => dv as c_int,
        Err(_) => -3,
    }
}

/// Classifica uma causa CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008.
///
/// # Safety
///
/// O ponteiro `cid_ptr` deve ser válido e terminado em null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_panama_classify_csap(cid_ptr: *const c_char) -> c_int {
    if cid_ptr.is_null() {
        return -1;
    }

    let c_str = unsafe { CStr::from_ptr(cid_ptr) };
    let cid = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    match classify_cid10(cid) {
        Some(group) => group.id() as c_int,
        None => 0,
    }
}

/// Projeta latitude e longitude em um índice hexagonal Uber H3.
#[unsafe(no_mangle)]
pub extern "C" fn brhealth_panama_latlng_to_h3(lat: f64, lng: f64, resolution: u8) -> u64 {
    coord_to_h3_index(lat, lng, resolution).unwrap_or_default()
}

/// Retorna a versão da biblioteca no buffer fornecido.
///
/// # Safety
///
/// O ponteiro `out_buf` deve ser válido e ter pelo menos `out_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn brhealth_panama_version(out_buf: *mut c_char, out_len: usize) -> c_int {
    if out_buf.is_null() || out_len < 8 {
        return -1;
    }

    let version_str = env!("CARGO_PKG_VERSION");
    let c_res = match CString::new(version_str) {
        Ok(c) => c,
        Err(_) => return -2,
    };
    let bytes = c_res.as_bytes_with_nul();
    if bytes.len() > out_len {
        return -3;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), out_buf, bytes.len());
    }
    0
}

/// Calcula o Retorno sobre Investimento (ROI) em Atenção Primária para Java Panama FFM.
#[unsafe(no_mangle)]
pub extern "C" fn brhealth_panama_compute_roi(
    avoidable_cost: f64,
    investment: f64,
    attributable_fraction: f64,
) -> f64 {
    brhealth_core::domain::analytics::csap::compute_primary_care_roi(
        avoidable_cost,
        investment,
        attributable_fraction,
    )
    .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panama_calculate_dv() {
        let code = CString::new("355030").unwrap();
        let dv = unsafe { brhealth_panama_calculate_dv(code.as_ptr()) };
        assert_eq!(dv, 8);
    }

    #[test]
    fn test_panama_classify_csap() {
        let cid = CString::new("J45").unwrap();
        let res = unsafe { brhealth_panama_classify_csap(cid.as_ptr()) };
        assert_eq!(res, 7); // Asma = Grupo 7
    }
}
