// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Submódulo de analítica avançada em saúde coletiva e bioestatística.

pub mod csap;
pub mod mortality;

pub use mortality::{
    ApvpMetrics, DEFAULT_CUTOFF_AGE_BR, DEFAULT_CUTOFF_AGE_WHO, WHO_STANDARD_POPULATION_WEIGHTS,
    compute_age_standardized_mortality_rate, compute_apvp, compute_apvp_rate, compute_batch_apvp,
};
