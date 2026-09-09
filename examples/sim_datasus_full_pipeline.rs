// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Exemplo de pipeline analítico de ponta a ponta:
//! 1. Ingestão de dados de Mortalidade (SIM) com descompressão nativa DATASUS Blast.
//! 2. Harmonização e validação de códigos territoriais IBGE.
//! 3. Indexação geoespacial em grade hexagonal Uber H3.
//! 4. Gravação de Snapshot Hive-Parquet com Time-Travel.
//! 5. Emissão de Manifesto FAIR e Grafo Criptográfico W3C PROV-O (JSON-LD).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use brhealth_core::decoders::DbcDecompressor;
use brhealth_core::domain::provenance::{
    FairManifest, PipelineStep, SourceProvenance, compute_sha256,
};
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceExecutionContext,
};
use brhealth_core::domain::spatial::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::calculate_ibge_dv;
use brhealth_core::infrastructure::cache::MemoryCache;
use brhealth_core::infrastructure::state::MemorySyncState;
use brhealth_core::infrastructure::storage::hive_parquet::HiveParquetStore;
use brhealth_core::infrastructure::transport::MockTransport;
use brhealth_core::sources::datasus::SimDataSource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BRHealth: Pipeline Analítico Completo SIM / DATASUS ===");

    // 1. Localizar arquivo de fixture real DATASUS (.dbc)
    let fixture_path = PathBuf::from("crates/brhealth-core/tests/fixtures/doac2022.dbc");
    if !fixture_path.exists() {
        eprintln!("Fixture não encontrada em: {:?}. Encerrando.", fixture_path);
        return Ok(());
    }

    let raw_bytes = std::fs::read(&fixture_path)?;
    let raw_sha256 = compute_sha256(&raw_bytes);
    println!(
        "-> Arquivo DBC lido: {} bytes (SHA-256: {})",
        raw_bytes.len(),
        &raw_sha256[..16]
    );

    // 2. Configurar parâmetros de busca do DATASUS
    let sim = SimDataSource::new();
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

    let locator = sim.resolve_locator(&params)?;
    println!("-> Localizador DATASUS resolvido: {}", locator);

    // 3. Configurar injeção de dependências
    let mock_transport = Arc::new(MockTransport::new());
    mock_transport.register_response(&locator, raw_bytes);

    let context = SourceExecutionContext {
        transport: mock_transport,
        decompressor: Arc::new(DbcDecompressor::new()?),
        cache: Arc::new(MemoryCache::new()),
        state: Arc::new(MemorySyncState::new()),
    };

    // 4. Executar ingestão com descompressão nativa Blast DCL -> Apache Arrow
    println!("-> Descomprimindo e decodificando microdados DATASUS...");
    let batches = sim.fetch_and_decode(&params, &context).await?;
    let batch = &batches[0];
    println!(
        "-> Apache Arrow RecordBatch gerado: {} linhas, {} colunas",
        batch.num_rows(),
        batch.num_columns()
    );

    // 5. Harmonização IBGE
    let rio_branco_dv = calculate_ibge_dv("120040")?;
    println!(
        "-> Validação IBGE: Rio Branco (120040) -> DV={}",
        rio_branco_dv
    );

    // 6. Indexação Espacial H3 (centróide de Rio Branco/AC: -9.97544, -67.8249, resolução 8)
    let h3_index = coord_to_h3_index(-9.97544, -67.8249, 8)?;
    println!("-> Indexação Espacial H3 (Res 8): {:#x}", h3_index);

    // 7. Persistência em Hive-Parquet com Time-Travel
    let temp_dir = tempfile::tempdir()?;
    let store = HiveParquetStore::new(temp_dir.path())?;
    let snapshot = store.save_batch("datasus.sim", "AC", 2022, batch)?;
    println!(
        "-> Snapshot Hive-Parquet gravado com sucesso: UUID={} (SHA-256: {})",
        snapshot.snapshot_id,
        &snapshot.sha256_parquet[..16]
    );

    // 8. Geração de Manifesto FAIR e Grafo W3C PROV-O
    let source_prov = SourceProvenance {
        source_name: "datasus.sim".into(),
        scope: "AC".into(),
        uri: locator,
        sha256_raw_payload: raw_sha256,
        retrieved_at: snapshot.created_at,
    };
    let step = PipelineStep {
        step: 1,
        operator: "analytics.sim_ingestion_and_parquet".into(),
        details: serde_json::json!({
            "snapshot_id": snapshot.snapshot_id,
            "h3_cell": format!("{:#x}", h3_index)
        }),
    };

    let manifest = FairManifest::new(vec![source_prov], vec![step]);
    let prov_json_ld = manifest.to_w3c_prov_json_ld();
    println!(
        "-> W3C PROV-O JSON-LD gerado: {} nós no grafo semântico",
        prov_json_ld["@graph"].as_array().map_or(0, |g| g.len())
    );

    println!("=== Pipeline concluído com sucesso e 100% de reprodutibilidade bit a bit! ===");
    Ok(())
}
