// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Testes de integração para a C-ABI plana do BRHealth.

use std::ffi::CString;

use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

use brhealth_ffi::{
    BRHEALTH_ERR_INVALID_ARG, BRHEALTH_ERR_NULL_PTR, BRHEALTH_SUCCESS, brhealth_calculate_ibge_dv,
    brhealth_classify_csap, brhealth_compute_primary_care_roi, brhealth_coord_to_h3,
    brhealth_export_arrow_batch, brhealth_version,
};

#[test]
fn test_c_abi_end_to_end() {
    // 1. Versão
    let ver_ptr = brhealth_version();
    assert!(!ver_ptr.is_null());

    // 2. IBGE
    let mut dv = 0u8;
    let code_sp = CString::new("355030").unwrap();
    assert_eq!(
        unsafe { brhealth_calculate_ibge_dv(code_sp.as_ptr(), &mut dv as *mut _) },
        BRHEALTH_SUCCESS
    );
    assert_eq!(dv, 8);

    // 3. H3
    let mut h3_idx = 0u64;
    assert_eq!(
        unsafe { brhealth_coord_to_h3(-23.55052, -46.633308, 8, &mut h3_idx as *mut _) },
        BRHEALTH_SUCCESS
    );
    assert!(h3_idx > 0);

    // 4. CSAP
    let mut group_id = 0u8;
    let cid = CString::new("I10").unwrap(); // Hipertensão (Grupo 9)
    assert_eq!(
        unsafe { brhealth_classify_csap(cid.as_ptr(), &mut group_id as *mut _) },
        BRHEALTH_SUCCESS
    );
    assert_eq!(group_id, 9);

    // 5. ROI
    let mut roi: f64 = 0.0;
    assert_eq!(
        unsafe {
            brhealth_compute_primary_care_roi(1_000_000.0, 200_000.0, 0.40, &mut roi as *mut _)
        },
        BRHEALTH_SUCCESS
    );
    assert!((roi - 1.0).abs() < 1e-6);

    // 6. Arrow C Data Interface
    let schema = Arc::new(Schema::new(vec![
        Field::new("cid", DataType::Utf8, false),
        Field::new("cost", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec!["I10"])),
            Arc::new(Float64Array::from(vec![500.0])),
        ],
    )
    .unwrap();

    let mut ffi_array = FFI_ArrowArray::empty();
    let mut ffi_schema = FFI_ArrowSchema::empty();

    assert_eq!(
        unsafe {
            brhealth_export_arrow_batch(
                &batch as *const _,
                &mut ffi_array as *mut _,
                &mut ffi_schema as *mut _,
            )
        },
        BRHEALTH_SUCCESS
    );
}

#[test]
fn test_c_abi_validation_errors() {
    let mut dv = 0u8;
    let invalid_code = CString::new("INVALID").unwrap();
    assert_eq!(
        unsafe { brhealth_calculate_ibge_dv(invalid_code.as_ptr(), &mut dv as *mut _) },
        BRHEALTH_ERR_INVALID_ARG
    );

    let mut h3 = 0u64;
    // Resolução inválida (> 15)
    assert_eq!(
        unsafe { brhealth_coord_to_h3(0.0, 0.0, 99, &mut h3 as *mut _) },
        BRHEALTH_ERR_INVALID_ARG
    );

    let mut roi = 0.0;
    // Investimento negativo
    assert_eq!(
        unsafe { brhealth_compute_primary_care_roi(100.0, -50.0, 0.5, &mut roi as *mut _) },
        BRHEALTH_ERR_INVALID_ARG
    );

    // Null pointer
    assert_eq!(
        unsafe { brhealth_calculate_ibge_dv(std::ptr::null(), &mut dv as *mut _) },
        BRHEALTH_ERR_NULL_PTR
    );
}
