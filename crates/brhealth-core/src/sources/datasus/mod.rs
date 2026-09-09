// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de dados para sistemas nacionais do DATASUS (Ministério da Saúde - Brasil).

pub mod helpers;
pub mod sih;
pub mod sim;
pub mod sinasc;

pub use sih::SihDataSource;
pub use sim::SimDataSource;
pub use sinasc::SinascDataSource;
