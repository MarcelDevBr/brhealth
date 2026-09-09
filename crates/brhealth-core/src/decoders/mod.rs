// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores decodificadores de dados brutos (Blast PKWARE DCL, DBF, GeoArrow, NetCDF, etc.)

pub mod blast;
pub mod dbc;
pub mod dbf;
pub mod geoarrow;
pub mod netcdf;

pub use blast::BlastDecompressor;
pub use dbc::DbcDecompressor;
pub use dbf::DbfDecoder;
pub use geoarrow::GeoArrowDecoder;
pub use netcdf::{ClimateGridVariable, NetCDFGridDecoder};
