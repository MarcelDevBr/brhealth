// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Interoperabilidade de Alta Performance Zero-Copy via Apache Arrow C Data Interface.
//!
//! Este módulo implementa a exportação e importação de lotes tabulares `RecordBatch`
//! através da especificação padronizada [Arrow C Data Interface](https://arrow.apache.org/docs/format/CDataInterface.html).
//! Permite travessia direta entre runtimes (Rust <-> Python via PyCapsule/ctypes, C++20 e Java 21+ FFM)
//! com **Zero-Copy real**: os ponteiros dos buffers físicos alinhados a 64 bytes são
//! transferidos diretamente sem serialização intermediária ou alocação redundante.

use arrow::array::{Array, ArrayData, StructArray};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema, from_ffi, to_ffi};
use arrow::record_batch::RecordBatch;

use crate::domain::ports::outbound::PortError;

/// Exporta um `RecordBatch` Apache Arrow para estruturas C da Arrow C Data Interface.
///
/// A transferência é estritamente Zero-Copy: os ponteiros dos buffers de memória subjacentes
/// são vinculados às estruturas C, e o destrutor (`release`) do Arrow garante que a memória
/// seja gerenciada corretamente pelo runtime receptor.
///
/// # Safety
///
/// Os ponteiros `out_array` e `out_schema` devem ser válidos e apontar para instâncias
/// alocadas de `FFI_ArrowArray` e `FFI_ArrowSchema`.
pub unsafe fn export_record_batch_to_c(
    batch: &RecordBatch,
    out_array: *mut FFI_ArrowArray,
    out_schema: *mut FFI_ArrowSchema,
) -> Result<(), PortError> {
    if out_array.is_null() || out_schema.is_null() {
        return Err(PortError::TransformationError(
            "Ponteiros out_array ou out_schema não podem ser nulos".into(),
        ));
    }

    let struct_array: StructArray = batch.clone().into();
    let data: ArrayData = struct_array.to_data();

    let (ffi_array, ffi_schema) =
        to_ffi(&data).map_err(|e| PortError::TransformationError(e.to_string()))?;

    // Copiar para os ponteiros fornecidos
    unsafe {
        std::ptr::write(out_array, ffi_array);
        std::ptr::write(out_schema, ffi_schema);
    }

    Ok(())
}

/// Importa um `RecordBatch` a partir de estruturas Arrow C Data Interface.
///
/// # Safety
///
/// As estruturas `array` e `schema` devem ser inicializadas conforme a especificação
/// Arrow C Data Interface.
pub unsafe fn import_record_batch_from_c(
    array: FFI_ArrowArray,
    schema: &FFI_ArrowSchema,
) -> Result<RecordBatch, PortError> {
    let data = unsafe {
        from_ffi(array, schema).map_err(|e| PortError::TransformationError(e.to_string()))?
    };

    let struct_array = StructArray::from(data);
    Ok(RecordBatch::from(&struct_array))
}

// ---------------------------------------------------------------------------
// C-ABI FFI estável (para consumo direto via dlopen / ctypes / JNI / C++)
// ---------------------------------------------------------------------------

/// Código de status retornado pela C-ABI em caso de sucesso.
pub const BRHEALTH_SUCCESS: i32 = 0;
/// Código de status para ponteiro nulo.
pub const BRHEALTH_ERR_NULL_POINTER: i32 = -1;
/// Código de status para falha interna de conversão.
pub const BRHEALTH_ERR_EXPORT_FAILED: i32 = -2;

/// Função interna C-ABI para exportar um lote `RecordBatch` para a Arrow C Data Interface.
///
/// # Safety
///
/// Requer ponteiros válidos e não-nulos para `batch_ptr`, `out_array` e `out_schema`.
pub unsafe extern "C" fn brhealth_export_arrow_batch(
    batch_ptr: *const RecordBatch,
    out_array: *mut FFI_ArrowArray,
    out_schema: *mut FFI_ArrowSchema,
) -> i32 {
    if batch_ptr.is_null() || out_array.is_null() || out_schema.is_null() {
        return BRHEALTH_ERR_NULL_POINTER;
    }

    let batch = unsafe { &*batch_ptr };
    match unsafe { export_record_batch_to_c(batch, out_array, out_schema) } {
        Ok(()) => BRHEALTH_SUCCESS,
        Err(_) => BRHEALTH_ERR_EXPORT_FAILED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, StringArray, UInt32Array};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    fn create_test_batch() -> RecordBatch {
        let cids = Arc::new(StringArray::from(vec!["J45", "I10", "A09"]));
        let codes = Arc::new(UInt32Array::from(vec![1200401, 3550308, 3304557]));
        let costs = Arc::new(Float64Array::from(vec![450.5, 320.0, 150.75]));

        let schema = Arc::new(Schema::new(vec![
            Field::new("cid", DataType::Utf8, false),
            Field::new("ibge_code", DataType::UInt32, false),
            Field::new("cost", DataType::Float64, false),
        ]));

        RecordBatch::try_new(schema, vec![cids, codes, costs]).unwrap()
    }

    #[test]
    fn test_zero_copy_c_data_interface_roundtrip() {
        let original_batch = create_test_batch();
        let num_rows = original_batch.num_rows();

        let mut ffi_array = FFI_ArrowArray::empty();
        let mut ffi_schema = FFI_ArrowSchema::empty();

        // 1. Exportar para C Data Interface
        unsafe {
            let res = export_record_batch_to_c(
                &original_batch,
                &mut ffi_array as *mut _,
                &mut ffi_schema as *mut _,
            );
            assert!(res.is_ok());
        }

        // 2. Importar de volta a partir de C Data Interface
        let imported_batch = unsafe { import_record_batch_from_c(ffi_array, &ffi_schema).unwrap() };

        assert_eq!(imported_batch.num_rows(), num_rows);
        assert_eq!(imported_batch.num_columns(), 3);

        // Validar integridade dos valores importados
        let cids = imported_batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(cids.value(0), "J45");
        assert_eq!(cids.value(1), "I10");
        assert_eq!(cids.value(2), "A09");
    }

    #[test]
    fn test_c_abi_export_function() {
        let batch = create_test_batch();
        let mut ffi_array = FFI_ArrowArray::empty();
        let mut ffi_schema = FFI_ArrowSchema::empty();

        // Testar exportação válida
        let status = unsafe {
            brhealth_export_arrow_batch(
                &batch as *const _,
                &mut ffi_array as *mut _,
                &mut ffi_schema as *mut _,
            )
        };
        assert_eq!(status, BRHEALTH_SUCCESS);

        // Testar rejeição de ponteiros nulos
        let status_null = unsafe {
            brhealth_export_arrow_batch(
                std::ptr::null(),
                &mut ffi_array as *mut _,
                &mut ffi_schema as *mut _,
            )
        };
        assert_eq!(status_null, BRHEALTH_ERR_NULL_POINTER);
    }
}
