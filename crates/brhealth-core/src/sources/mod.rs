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
    BpsDataSource, CnesDataSource, SiasusDataSource, SihDataSource, SimDataSource,
    SinanDataSource, SinascDataSource, SipniDataSource, SiscanDataSource, SisvanDataSource,
};
pub use environmental::{BdQueimadasDataSource, InmetDataSource, SisaguaDataSource};
pub use global::{
    Era5DataSource, IhmeGbdDataSource, PahoPlisaDataSource, WhoGhoDataSource, WorldPopDataSource,
};
pub use ibge::{IbgeCensoDataSource, IbgePnadDataSource};
pub use mds::CadUnicoDataSource;

use crate::domain::source_spi::HealthDataSourceSPI;

/// Constrói e retorna o conjunto completo de fontes nacionais do **Country Pack Brasil** (`pack_br`).
///
/// Inclui as fontes prioritárias do SUS (SIM, SINASC, SIH, SINAN, SIASUS, CNES, SIPNI, SISVAN, SISCAN, BPS),
/// estatísticas do IBGE (Censo e PNAD), vulnerabilidade do MDS (CadÚnico) e monitoramento socioambiental
/// (INMET, BDQueimadas e SISAGUA).
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
        // 2. IBGE - Demografia e Renda
        Arc::new(IbgeCensoDataSource::new()),
        Arc::new(IbgePnadDataSource::new()),
        // 3. MDS - Vulnerabilidade Social
        Arc::new(CadUnicoDataSource::new()),
        // 4. Clima, Ambiente e Saneamento
        Arc::new(InmetDataSource::new()),
        Arc::new(BdQueimadasDataSource::new()),
        Arc::new(SisaguaDataSource::new()),
    ]
}

/// Constrói e retorna o conjunto completo de fontes supranacionais do **Country Pack Global** (`pack_global`).
///
/// Inclui indicadores globais da OMS (WHO GHO), estudos de carga de doença (IHME GBD),
/// reanálise climática em grade (Copernicus ERA5), grades populacionais (WorldPop) e
/// vigilância pan-americana de arboviroses (PAHO / PLISA).
#[must_use]
pub fn create_pack_global() -> Vec<Arc<dyn HealthDataSourceSPI>> {
    vec![
        Arc::new(WhoGhoDataSource::new()),
        Arc::new(IhmeGbdDataSource::new()),
        Arc::new(Era5DataSource::new()),
        Arc::new(WorldPopDataSource::new()),
        Arc::new(PahoPlisaDataSource::new()),
    ]
}
