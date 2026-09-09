// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores para dados ambientais, climáticos e de saneamento.

pub mod bdqueimadas;
pub mod inmet;
pub mod sisagua;

pub use bdqueimadas::BdQueimadasDataSource;
pub use inmet::InmetDataSource;
pub use sisagua::SisaguaDataSource;
