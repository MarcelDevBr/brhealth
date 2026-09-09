// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Módulo de computação espacial discreta e georreferenciamento de eventos de saúde.

pub mod h3;

pub use h3::{
    DEFAULT_HIGH_PRECISION_RESOLUTION, DEFAULT_INTRAURBAN_RESOLUTION, DEFAULT_MUNICIPAL_RESOLUTION,
    append_h3_column, coord_to_h3_index, h3_grid_disk, h3_grid_distance, h3_index_to_coord,
};
