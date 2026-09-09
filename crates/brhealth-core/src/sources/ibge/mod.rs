// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores para pesquisas estatísticas e censitárias do IBGE.

pub mod censo;
pub mod munic;
pub mod pense;
pub mod pnad;
pub mod pof;

pub use censo::IbgeCensoDataSource;
pub use munic::IbgeMunicDataSource;
pub use pense::IbgePenseDataSource;
pub use pnad::IbgePnadDataSource;
pub use pof::IbgePofDataSource;
