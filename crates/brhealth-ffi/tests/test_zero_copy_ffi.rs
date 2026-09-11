// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Testes de integração para interoperabilidade FFI Zero-Copy com Apache Arrow C Data Interface.

use std::sync::Arc;

use arrow::array::{Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use arrow::record_batch::RecordBatch;

use brhealth_ffi::{
    BRHEALTH_ERR_NULL_PTR, BRHEALTH_SUCCESS, brhealth_export_arrow_batch, export_record_batch_to_c,
    import_record_batch_from_c,
};

#[test]
fn test_ffi_zero_copy_roundtrip_with_large_payload() {
    let num_rows = 10_000;
    let mut names = Vec::with_capacity(num_rows);
    let mut counts = Vec::with_capacity(num_rows);
    let mut values = Vec::with_capacity(num_rows);

    for i in 0..num_rows {
        names.push(Some("CSAP_TEST_DIAGNOSIS"));
        counts.push(Some(i as u64));
        values.push(Some((i as f64) * 1.5));
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("diagnosis", DataType::Utf8, true),
        Field::new("row_id", DataType::UInt64, true),
        Field::new("cost", DataType::Float64, true),
    ]));

    let original_batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(names)),
            Arc::new(UInt64Array::from(counts)),
            Arc::new(Float64Array::from(values)),
        ],
    )
    .unwrap();

    let mut ffi_array = FFI_ArrowArray::empty();
    let mut ffi_schema = FFI_ArrowSchema::empty();

    // Exportação C Data Interface
    unsafe {
        let export_res = export_record_batch_to_c(
            &original_batch,
            &mut ffi_array as *mut _,
            &mut ffi_schema as *mut _,
        );
        assert!(export_res.is_ok());
    }

    // Importação C Data Interface (simulando recepção por processo Python / C++)
    let imported_batch = unsafe { import_record_batch_from_c(ffi_array, &ffi_schema).unwrap() };

    assert_eq!(imported_batch.num_rows(), num_rows);
    assert_eq!(imported_batch.num_columns(), 3);

    // Validar primeiro e último registro
    let count_col = imported_batch
        .column(1)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .unwrap();

    assert_eq!(count_col.value(0), 0);
    assert_eq!(count_col.value(num_rows - 1), (num_rows - 1) as u64);
}

#[test]
fn test_c_abi_export_error_conditions() {
    let mut ffi_array = FFI_ArrowArray::empty();
    let mut ffi_schema = FFI_ArrowSchema::empty();

    // 0. Exportação válida vazia
    let batch_empty = RecordBatch::new_empty(Arc::new(Schema::empty()));
    let res0 = unsafe {
        brhealth_export_arrow_batch(
            &batch_empty as *const _,
            &mut ffi_array as *mut _,
            &mut ffi_schema as *mut _,
        )
    };
    assert_eq!(res0, BRHEALTH_SUCCESS);

    // 1. Passar ponteiro nulo para o lote
    let res1 = unsafe {
        brhealth_export_arrow_batch(
            std::ptr::null(),
            &mut ffi_array as *mut _,
            &mut ffi_schema as *mut _,
        )
    };
    assert_eq!(res1, BRHEALTH_ERR_NULL_PTR);

    // 2. Passar ponteiro nulo para out_array
    let batch = RecordBatch::new_empty(Arc::new(Schema::empty()));
    let res2 = unsafe {
        brhealth_export_arrow_batch(
            &batch as *const _,
            std::ptr::null_mut(),
            &mut ffi_schema as *mut _,
        )
    };
    assert_eq!(res2, BRHEALTH_ERR_NULL_PTR);

    // 3. Passar ponteiro nulo para out_schema
    let res3 = unsafe {
        brhealth_export_arrow_batch(
            &batch as *const _,
            &mut ffi_array as *mut _,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(res3, BRHEALTH_ERR_NULL_PTR);
}
