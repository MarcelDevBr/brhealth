// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Módulo de Foreign Function Interface (FFI) Zero-Copy.

pub mod c_data;

pub use c_data::{
    BRHEALTH_ERR_EXPORT_FAILED, BRHEALTH_ERR_NULL_POINTER, BRHEALTH_SUCCESS,
    brhealth_export_arrow_batch, export_record_batch_to_c, import_record_batch_from_c,
};
