// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores para dados supranacionais e globais (Country Pack Global).

pub mod era5;
pub mod ihme_gbd;
pub mod paho_plisa;
pub mod who_gho;
pub mod worldpop;

pub use era5::Era5DataSource;
pub use ihme_gbd::IhmeGbdDataSource;
pub use paho_plisa::PahoPlisaDataSource;
pub use who_gho::WhoGhoDataSource;
pub use worldpop::WorldPopDataSource;
