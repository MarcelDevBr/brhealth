// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Decodificador colunar de tabelas legadas dBase / DBF para o padrão Apache Arrow.

pub mod header;
pub mod reader;

pub use header::{DbfFieldDescriptor, DbfFieldType, DbfHeader};
pub use reader::DbfDecoder;
