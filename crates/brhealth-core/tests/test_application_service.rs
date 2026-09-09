// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use brhealth_core::decoders::dbc::DbcDecompressor;
use brhealth_core::domain::application::{
    BRHealthApplicationService, PipelineExecutionOptions,
};
use brhealth_core::domain::ports::outbound::{
    LocalCachePort, PortError, SyncStatePort, TransportPort,
};
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, SourceExecutionContext,
};
use brhealth_core::infrastructure::state::disk::DiskSyncState;
use brhealth_core::sources::datasus::SimDataSource;
use brhealth_core::SourceRegistry;

struct FixtureTransport {
    data: Vec<u8>,
}

#[async_trait::async_trait]
impl TransportPort for FixtureTransport {
    async fn fetch_bytes(&self, _uri: &str) -> Result<Vec<u8>, PortError> {
        Ok(self.data.clone())
    }
}

struct MockCache;

impl LocalCachePort for MockCache {
    fn exists(&self, _key: &str) -> bool {
        false
    }
    fn read(&self, _key: &str) -> Result<Vec<u8>, PortError> {
        Err(PortError::CacheError("Miss".into()))
    }
    fn write(&self, _key: &str, _data: &[u8]) -> Result<(), PortError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_application_service_full_pipeline_with_sim() {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push("tests/fixtures/doac2022.dbc");

    if !fixture_path.exists() {
        return;
    }

    let compressed_bytes = fs::read(&fixture_path).expect("Falha ao ler fixture");

    let tmp = tempdir().unwrap();
    let state_file = tmp.path().join("state.db");
    let sync_state = Arc::new(DiskSyncState::open(&state_file).unwrap());

    let mut registry = SourceRegistry::new();
    registry.register(SimDataSource::new());

    let context = Arc::new(SourceExecutionContext {
        transport: Arc::new(FixtureTransport { data: compressed_bytes }),
        decompressor: Arc::new(DbcDecompressor::new().unwrap()),
        cache: Arc::new(MockCache),
        state: sync_state.clone() as Arc<dyn SyncStatePort>,
    });

    let app_service =
        BRHealthApplicationService::new(Arc::new(registry), context, sync_state);

    let params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".into(),
        },
        jurisdiction_code: Some("AC".into()),
        year: 2022,
        month: None,
        extra_filters: HashMap::new(),
        as_of_snapshot: None,
    };

    let cache_dir = tmp.path().join("hive_cache");
    let options = PipelineExecutionOptions {
        harmonize_ibge: true,
        assign_h3_resolution: None,
        h3_coord_columns: None,
        enrich_csap: false,
        reference_population: Some(830_000),
        persist_to_cache: true,
        cache_base_path: Some(cache_dir.clone()),
    };

    let result = app_service
        .execute_full_pipeline("datasus.sim", &params, &options)
        .await
        .unwrap();

    assert!(!result.manifest.sources.is_empty());
    assert_eq!(result.batches.len(), 1);
    assert!(result.batches[0].num_rows() > 0);
    assert!(result.persisted_snapshot_id.is_some());
    assert!(cache_dir.exists());
}

