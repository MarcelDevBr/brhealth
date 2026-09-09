// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de dados para sistemas nacionais do DATASUS (Ministério da Saúde - Brasil).

pub mod bps;
pub mod cnes;
pub mod helpers;
pub mod siasus;
pub mod sih;
pub mod sim;
pub mod sinan;
pub mod sinasc;
pub mod sipni;
pub mod siscan;
pub mod sisvan;

pub use bps::BpsDataSource;
pub use cnes::CnesDataSource;
pub use siasus::SiasusDataSource;
pub use sih::SihDataSource;
pub use sim::SimDataSource;
pub use sinan::SinanDataSource;
pub use sinasc::SinascDataSource;
pub use sipni::SipniDataSource;
pub use siscan::SiscanDataSource;
pub use sisvan::SisvanDataSource;
