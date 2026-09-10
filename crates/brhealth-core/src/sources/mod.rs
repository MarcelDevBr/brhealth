// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Pacotes de fontes regionais e globais (Country Packs).

pub mod datasus;
pub mod environmental;
pub mod global;
pub mod ibge;
pub mod mds;

use std::sync::Arc;

pub use datasus::{
    BpsDataSource, CnesDataSource, SiasusDataSource, SihDataSource, SimDataSource, SinanDataSource,
    SinascDataSource, SipniDataSource, SiscanDataSource, SisvanDataSource,
};
pub use environmental::{
    BdQueimadasDataSource, InmetDataSource, ProdesDataSource, SisaguaDataSource,
};
pub use global::{
    Era5DataSource, IhmeGbdDataSource, OpenAqDataSource, PahoPlisaDataSource, WhoGhoDataSource,
    WorldPopDataSource,
};
pub use ibge::{
    IbgeCensoDataSource, IbgeMunicDataSource, IbgePenseDataSource, IbgePnadDataSource,
    IbgePofDataSource,
};
pub use mds::CadUnicoDataSource;

use crate::domain::source_spi::HealthDataSourceSPI;

/// Constrói e retorna o conjunto completo de fontes nacionais do **Country Pack Brasil** (`pack_br`).
#[must_use]
pub fn create_pack_brasil() -> Vec<Arc<dyn HealthDataSourceSPI>> {
    vec![
        // 1. DATASUS - Estatísticas Vitais e Assistenciais
        Arc::new(SimDataSource::new()),
        Arc::new(SinascDataSource::new()),
        Arc::new(SihDataSource::new()),
        Arc::new(SinanDataSource::new()),
        Arc::new(SiasusDataSource::new()),
        Arc::new(CnesDataSource::new()),
        Arc::new(SipniDataSource::new()),
        Arc::new(SisvanDataSource::new()),
        Arc::new(SiscanDataSource::new()),
        Arc::new(BpsDataSource::new()),
        // 2. IBGE - Demografia, Condições de Vida, Orçamentos e Gestão
        Arc::new(IbgeCensoDataSource::new()),
        Arc::new(IbgePnadDataSource::new()),
        Arc::new(IbgePofDataSource::new()),
        Arc::new(IbgePenseDataSource::new()),
        Arc::new(IbgeMunicDataSource::new()),
        // 3. MDS - Vulnerabilidade Social
        Arc::new(CadUnicoDataSource::new()),
        // 4. Clima, Ambiente e Desmatamento
        Arc::new(InmetDataSource::new()),
        Arc::new(BdQueimadasDataSource::new()),
        Arc::new(ProdesDataSource::new()),
        Arc::new(SisaguaDataSource::new()),
    ]
}

/// Constrói e retorna o conjunto completo de fontes supranacionais do **Country Pack Global** (`pack_global`).
#[must_use]
pub fn create_pack_global() -> Vec<Arc<dyn HealthDataSourceSPI>> {
    vec![
        Arc::new(WhoGhoDataSource::new()),
        Arc::new(IhmeGbdDataSource::new()),
        Arc::new(Era5DataSource::new()),
        Arc::new(WorldPopDataSource::new()),
        Arc::new(PahoPlisaDataSource::new()),
        Arc::new(OpenAqDataSource::new()),
    ]
}
