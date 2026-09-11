// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Interoperabilidade de Alta Performance Zero-Copy via Apache Arrow C Data Interface.
//!
//! Este módulo implementa a exportação e importação de lotes tabulares `RecordBatch`
//! através da especificação padronizada [Arrow C Data Interface](https://arrow.apache.org/docs/format/CDataInterface.html).
//! Permite travessia direta entre runtimes (Rust <-> C, C++20, Python via ctypes/cffi, Zig e Java 21+ FFM)
//! com **Zero-Copy real**: os ponteiros dos buffers físicos alinhados a 64 bytes são
//! transferidos diretamente sem serialização intermediária ou alocação redundante.

use arrow::array::{Array, ArrayData, StructArray};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema, from_ffi, to_ffi};
use arrow::record_batch::RecordBatch;
use brhealth_core::PortError;

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
