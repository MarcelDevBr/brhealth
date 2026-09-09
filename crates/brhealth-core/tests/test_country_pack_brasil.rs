// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use brhealth_core::decoders::DbcDecompressor;
use brhealth_core::domain::schema::CanonicalSchemas;
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceExecutionContext,
};
use brhealth_core::infrastructure::{MemoryCache, MemorySyncState, MockTransport};
use brhealth_core::sources::{SihDataSource, SimDataSource, SinascDataSource};

#[tokio::test]
async fn test_sim_end_to_end_ingestion_with_fixture() {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push("tests/fixtures/doac2022.dbc");

    if !fixture_path.exists() {
        eprintln!(
            "Fixture não encontrada em: {:?}, pulando teste",
            fixture_path
        );
        return;
    }

    let compressed_bytes = fs::read(&fixture_path).expect("Falha ao ler fixture doac2022.dbc");

    let sim_source = SimDataSource::new();
    let params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".to_string(),
        },
        jurisdiction_code: Some("AC".to_string()),
        year: 2022,
        month: None,
        extra_filters: HashMap::new(),
        as_of_snapshot: None,
    };

    let locator = sim_source.resolve_locator(&params).unwrap();
    assert!(locator.contains("DOAC2022.dbc"));

    // Monta contexto com MockTransport simulando o FTP do DATASUS
    let mock_transport = Arc::new(MockTransport::new());
    mock_transport.register_response(&locator, compressed_bytes);

    let context = SourceExecutionContext {
        transport: mock_transport,
        decompressor: Arc::new(DbcDecompressor::new().unwrap()),
        cache: Arc::new(MemoryCache::new()),
        state: Arc::new(MemorySyncState::new()),
    };

    let batches = sim_source
        .fetch_and_decode(&params, &context)
        .await
        .unwrap();
    assert_eq!(batches.len(), 1);

    let canonical_batch = &batches[0];
    assert_eq!(canonical_batch.num_rows(), 4159);
    assert_eq!(
        canonical_batch.schema(),
        CanonicalSchemas::canonical_mortality_schema()
    );

    // Valida colunas canônicas
    let country_col = canonical_batch.column(1);
    assert_eq!(country_col.len(), 4159);

    let juris_col = canonical_batch.column(2);
    assert_eq!(juris_col.len(), 4159);
}

#[test]
fn test_sinasc_and_sih_metadata_and_locators() {
    let sinasc = SinascDataSource::new();
    assert_eq!(sinasc.metadata().id, "datasus.sinasc");
    assert_eq!(
        sinasc.target_schema(),
        CanonicalSchemas::canonical_birth_schema()
    );

    let sinasc_params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".to_string(),
        },
        jurisdiction_code: Some("SP".to_string()),
        year: 2023,
        month: None,
        extra_filters: HashMap::new(),
        as_of_snapshot: None,
    };
    let sinasc_loc = sinasc.resolve_locator(&sinasc_params).unwrap();
    assert!(sinasc_loc.contains("DNSP2023.dbc"));

    let sih = SihDataSource::new();
    assert_eq!(sih.metadata().id, "datasus.sih");
    assert_eq!(
        sih.target_schema(),
        CanonicalSchemas::canonical_hospital_morbidity_schema()
    );

    let sih_params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".to_string(),
        },
        jurisdiction_code: Some("MG".to_string()),
        year: 2023,
        month: Some(7),
        extra_filters: HashMap::new(),
        as_of_snapshot: None,
    };
    let sih_loc = sih.resolve_locator(&sih_params).unwrap();
    assert!(sih_loc.contains("RDMG2307.dbc"));
}
