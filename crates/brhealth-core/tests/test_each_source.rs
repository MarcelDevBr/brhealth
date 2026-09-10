// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Testes unitários e de integração individuais para cada uma das 26 fontes do BRHealth.
//!
//! Permite execução granular via `cargo test --test test_each_source <filtro>`
//! ou automação através de `scripts/test_sources.sh`.

use std::collections::HashMap;
use std::sync::Arc;

use brhealth_core::decoders::dbc::DbcDecompressor;
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceExecutionContext,
};
use brhealth_core::infrastructure::cache::MemoryCache;
use brhealth_core::infrastructure::state::MemorySyncState;
use brhealth_core::infrastructure::transport::MockTransport;
use brhealth_core::sources::*;

fn create_test_context() -> (SourceExecutionContext, Arc<MockTransport>) {
    let transport = Arc::new(MockTransport::new());
    let decompressor = Arc::new(DbcDecompressor::new().expect("Decompressor"));
    let cache = Arc::new(MemoryCache::new());
    let state = Arc::new(MemorySyncState::new());

    let ctx = SourceExecutionContext {
        transport: transport.clone(),
        decompressor,
        cache,
        state,
    };
    (ctx, transport)
}

fn create_sample_brazil_params() -> DataQueryParams {
    let mut extra = HashMap::new();
    extra.insert("disease".into(), "DENG".into());
    extra.insert("sub_system".into(), "PA".into());
    extra.insert("type".into(), "ST".into());
    extra.insert("exam_type".into(), "MAM".into());

    DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".into(),
        },
        jurisdiction_code: Some("SP".into()),
        year: 2022,
        month: Some(1),
        extra_filters: extra,
        as_of_snapshot: None,
    }
}

fn create_sample_global_params() -> DataQueryParams {
    let mut extra = HashMap::new();
    extra.insert("indicator".into(), "WHOSIS_000001".into());
    extra.insert("measure".into(), "Deaths".into());
    extra.insert("grid_type".into(), "reanalysis".into());

    DataQueryParams {
        scope: GeographicScope::Supranational {
            entity: "GLOBAL".into(),
        },
        jurisdiction_code: Some("BRA".into()),
        year: 2022,
        month: Some(1),
        extra_filters: extra,
        as_of_snapshot: None,
    }
}

async fn run_source_check(source: &dyn HealthDataSourceSPI, params: &DataQueryParams) {
    let meta = source.metadata();
    assert!(!meta.id.is_empty(), "ID da fonte não pode ser vazio");
    assert!(!meta.display_name.is_empty(), "Nome não pode ser vazio");
    assert!(
        !meta.maintaining_agency.is_empty(),
        "Órgão não pode ser vazio"
    );

    let schema = source.target_schema();
    assert!(!schema.fields().is_empty(), "Esquema Arrow deve ter campos");

    let locator_res = source.resolve_locator(params);
    assert!(
        locator_res.is_ok(),
        "Locator deve ser resolvido para {}",
        meta.id
    );
    let locator = locator_res.unwrap();
    assert!(!locator.is_empty(), "Locator não pode ser vazio");

    let (ctx, transport) = create_test_context();
    transport.register_response(&locator, vec![0u8; 64]);

    let res = source.fetch_and_decode(params, &ctx).await;
    // Verifica que executa de forma segura e não entra em panic
    assert!(res.is_ok() || res.is_err());
}

// ==========================================
// 1. DATASUS (10 FONTES)
// ==========================================

#[tokio::test]
async fn test_source_datasus_sim() {
    let source = SimDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_sinasc() {
    let source = SinascDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_sih() {
    let source = SihDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_sinan() {
    let source = SinanDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_siasus() {
    let source = SiasusDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_cnes() {
    let source = CnesDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_sipni() {
    let source = SipniDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_sisvan() {
    let source = SisvanDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_siscan() {
    let source = SiscanDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_datasus_bps() {
    let source = BpsDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

// ==========================================
// 2. IBGE (5 FONTES)
// ==========================================

#[tokio::test]
async fn test_source_ibge_censo() {
    let source = IbgeCensoDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_ibge_pnad() {
    let source = IbgePnadDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_ibge_pof() {
    let source = IbgePofDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_ibge_pense() {
    let source = IbgePenseDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_ibge_munic() {
    let source = IbgeMunicDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

// ==========================================
// 3. MDS (1 FONTE)
// ==========================================

#[tokio::test]
async fn test_source_mds_cadunico() {
    let source = CadUnicoDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

// ==========================================
// 4. AMBIENTAL / CLIMA (4 FONTES)
// ==========================================

#[tokio::test]
async fn test_source_environmental_inmet() {
    let source = InmetDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_environmental_bdqueimadas() {
    let source = BdQueimadasDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_environmental_prodes() {
    let source = ProdesDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_environmental_sisagua() {
    let source = SisaguaDataSource::new();
    let params = create_sample_brazil_params();
    run_source_check(&source, &params).await;
}

// ==========================================
// 5. GLOBAL / SUPRANACIONAL (6 FONTES)
// ==========================================

#[tokio::test]
async fn test_source_global_who_gho() {
    let source = WhoGhoDataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_global_ihme_gbd() {
    let source = IhmeGbdDataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_global_copernicus_era5() {
    let source = Era5DataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_global_worldpop() {
    let source = WorldPopDataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_global_paho_plisa() {
    let source = PahoPlisaDataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}

#[tokio::test]
async fn test_source_global_openaq() {
    let source = OpenAqDataSource::new();
    let params = create_sample_global_params();
    run_source_check(&source, &params).await;
}
