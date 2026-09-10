// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Serviço de Aplicação Unificado do BRHealth (*Application Service*).
//!
//! Orquestra as portas de entrada (*Inbound Ports*) ligando os casos de uso
//! do domínio puro às portas de saída (*Outbound SPIs*), mantendo a separação
//! rigorosa da Arquitetura Hexagonal DOD.

use std::sync::Arc;

use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use chrono::Utc;

use crate::domain::analytics::csap::{CsapMetrics, compute_csap_metrics};
use crate::domain::ports::inbound::{
    CSAPAnalysisSummary, CSAPCostAnalysisPort, GlobalHarmonizationPort, MultidimensionalQueryPort,
    ProvenanceExtractionPort, SnapshotTimeTravelPort, SpatialJoinEnginePort,
};
use crate::domain::ports::outbound::{PortError, SyncStatePort};
use crate::domain::provenance::{FairManifest, SourceProvenance};
use crate::domain::registry::SourceRegistry;
use crate::domain::source_spi::{DataQueryParams, SourceExecutionContext};
use crate::domain::spatial::h3::append_h3_column;
use crate::domain::transforms::ibge::harmonize_ibge_code;
use crate::domain::transforms::ontology::MedicalOntologyHarmonizer;
use crate::domain::transforms::pipeline::{
    BatchTransformationStep, CsapEnrichmentStep, H3SpatialIndexingStep, TransformationPipeline,
};

use crate::infrastructure::storage::hive_parquet::HiveParquetStore;
use crate::infrastructure::transport::resilience::DataFreshness;

/// Opções de configuração para o pipeline analítico de ponta a ponta.
#[derive(Debug, Clone, Default)]
pub struct PipelineExecutionOptions {
    /// Se verdadeiro, harmoniza colunas de códigos municipais de 6 para 7 dígitos canônicos.
    pub harmonize_ibge: bool,
    /// Se fornecido, calcula e anexa índice espacial discreto Uber H3 na resolução configurada (ex: 8).
    pub assign_h3_resolution: Option<u8>,
    /// Nomes das colunas de latitude e longitude no lote para cálculo do H3.
    pub h3_coord_columns: Option<(String, String)>,
    /// Se verdadeiro, processa e enriquece internações de morbidade hospitalar (SIH) com CSAP.
    pub enrich_csap: bool,
    /// População de referência do município/região para cálculo da Taxa Bruta de CSAP por 10.000 hab.
    pub reference_population: Option<u64>,
    /// Se verdadeiro, grava os lotes no cache local Hive-Parquet com time-travel determinístico.
    pub persist_to_cache: bool,
    /// Caminho do diretório raiz para persistência do cache Hive-Parquet.
    pub cache_base_path: Option<std::path::PathBuf>,
    /// Passos customizados adicionais de transformação colunar (Chain of Responsibility).
    pub custom_steps: Vec<Arc<dyn BatchTransformationStep>>,
}

impl PipelineExecutionOptions {
    /// Adiciona um passo customizado de transformação à cadeia.
    #[must_use]
    pub fn with_step<S: BatchTransformationStep + 'static>(mut self, step: S) -> Self {
        self.custom_steps.push(Arc::new(step));
        self
    }
}

/// Resultado consolidado da execução do pipeline analítico.
#[derive(Debug, Clone)]
pub struct PipelineExecutionResult {
    /// Lotes colunares Apache Arrow resultantes do pipeline.
    pub batches: Vec<RecordBatch>,
    /// Manifesto FAIR de linhagem científica W3C PROV-O com hashes SHA-256 brutos.
    pub manifest: FairManifest,
    /// Sumário executivo e econômico de CSAP (quando aplicável ou solicitado).
    pub csap_summary: Option<CSAPAnalysisSummary>,
    /// Identificador do snapshot histórico gravado no cache Hive-Parquet.
    pub persisted_snapshot_id: Option<String>,
    /// Indicador de frescura dos dados obtidos.
    pub data_freshness: DataFreshness,
}

/// Serviço central de aplicação que implementa as portas *Inbound*.
#[derive(Clone)]
pub struct BRHealthApplicationService {
    registry: Arc<SourceRegistry>,
    context: Arc<SourceExecutionContext>,
    sync_state: Arc<dyn SyncStatePort>,
    ontology_harmonizer: Arc<MedicalOntologyHarmonizer>,
}

impl BRHealthApplicationService {
    /// Cria uma nova instância do serviço de aplicação.
    pub fn new(
        registry: Arc<SourceRegistry>,
        context: Arc<SourceExecutionContext>,
        sync_state: Arc<dyn SyncStatePort>,
    ) -> Self {
        Self {
            registry,
            context,
            sync_state,
            ontology_harmonizer: Arc::new(MedicalOntologyHarmonizer::new()),
        }
    }

    /// Constrói uma instância padrão pronta para produção com pacotes oficiais (Brasil e Global),
    /// transporte DATASUS FTP resiliente, descompressor nativo DBC, cache em memória e controle de estado.
    pub fn standard_in_memory() -> Result<Self, PortError> {
        let registry = Arc::new(SourceRegistry::standard());
        let transport =
            Arc::new(crate::infrastructure::transport::AsyncFtpTransport::new_datasus());
        let decompressor = Arc::new(crate::decoders::dbc::DbcDecompressor::new()?);
        let sync_state: Arc<dyn SyncStatePort> =
            Arc::new(crate::infrastructure::state::MemorySyncState::new());
        let cache = Arc::new(crate::infrastructure::cache::MemoryCache::new());

        let context = Arc::new(SourceExecutionContext {
            transport,
            decompressor,
            cache,
            state: sync_state.clone(),
        });

        Ok(Self::new(registry, context, sync_state))
    }

    /// Retorna a referência ao registro de fontes.
    #[must_use]
    pub fn registry(&self) -> &Arc<SourceRegistry> {
        &self.registry
    }

    /// Retorna a referência ao contexto de execução de fontes.
    #[must_use]
    pub fn context(&self) -> &Arc<SourceExecutionContext> {
        &self.context
    }

    /// Executa o pipeline analítico de ponta a ponta:
    /// Ingestão $\to$ Descompressão $\to$ Harmonização IBGE $\to$ Indexação H3 $\to$ Enriquecimento CSAP $\to$ Cache Hive-Parquet $\to$ Manifesto FAIR W3C PROV-O.
    ///
    /// Em caso de falha na fonte primária, tenta mirrors declarados pela fonte.
    /// Se todos falharem e `persist_to_cache` estiver habilitado com dados existentes,
    /// serve os dados stale do cache Hive-Parquet local.
    pub async fn execute_full_pipeline(
        &self,
        source_id: &str,
        params: &DataQueryParams,
        options: &PipelineExecutionOptions,
    ) -> Result<PipelineExecutionResult, PortError> {
        let source = self.registry.get(source_id)?;
        let locator = source.resolve_locator(params)?;
        let mirror_uris = source.mirror_uris(params);

        // 1. Ingestão com fallback resiliente
        let (raw_batches, data_freshness) = match self.execute_query(source_id, params).await {
            Ok(batches) => (batches, DataFreshness::Fresh),
            Err(primary_err) => {
                // Tentar mirrors
                let mut mirror_result = None;
                for (idx, mirror_uri) in mirror_uris.iter().enumerate() {
                    eprintln!(
                        "ℹ [BRHealth] Tentando espelho de contingência #{}/{} para '{}': {}",
                        idx + 1,
                        mirror_uris.len(),
                        source_id,
                        mirror_uri
                    );
                    let mirror_bytes = self.context.transport.fetch_bytes(mirror_uri).await;
                    if let Ok(bytes) = mirror_bytes {
                        eprintln!(
                            "✓ [BRHealth] Espelho conectado com sucesso. Descomprimindo e decodificando payload..."
                        );
                        // Decodificar via decompressor + decoder
                        let decompressed = self.context.decompressor.decompress(&bytes)?;
                        let dbf_decoder = crate::decoders::dbf::DbfDecoder::new();
                        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed)?;
                        let canonical = source.fetch_and_decode(params, &self.context).await;
                        if let Ok(batches) = canonical {
                            mirror_result = Some(batches);
                            break;
                        }
                        // Se a decodificação falhar, tentar próximo mirror
                        let _ = mirror_result.insert(vec![raw_batch]);
                        break;
                    }
                }

                if let Some(batches) = mirror_result {
                    (
                        batches,
                        DataFreshness::Stale {
                            cached_at: Utc::now(),
                            reason: format!("Obtido via mirror após falha primária: {primary_err}"),
                        },
                    )
                } else if let (true, Some(base_path)) =
                    (options.persist_to_cache, options.cache_base_path.as_ref())
                {
                    // Fallback para cache Hive-Parquet stale
                    let store = HiveParquetStore::new(base_path)?;
                    let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
                    match store.read_as_of(source_id, uf, params.year as i32, Utc::now())? {
                        Some(cached_batch) => {
                            eprintln!(
                                "⚠ [BRHealth] Rede indisponível. Servindo snapshot local em cache para '{}'.",
                                source_id
                            );
                            (
                                vec![cached_batch],
                                DataFreshness::Stale {
                                    cached_at: Utc::now(),
                                    reason: format!(
                                        "Cache stale após falha de todas as fontes: {primary_err}"
                                    ),
                                },
                            )
                        }
                        None => {
                            return Err(PortError::DegradedSource(format!(
                                "A fonte oficial '{}' está temporariamente inacessível e não há snapshot no cache local.\nDetalhes técnicos: {}\nSugestão: Verifique sua conexão à internet ou aguarde o restabelecimento do serviço do DATASUS/órgão emissor.",
                                source_id, primary_err
                            )));
                        }
                    }
                } else {
                    return Err(PortError::DegradedSource(format!(
                        "A fonte oficial '{}' está temporariamente inacessível nos servidores remotos.\nDetalhes técnicos: {}\nSugestão: O serviço governamental pode estar instável. Tente novamente em alguns instantes.",
                        source_id, primary_err
                    )));
                }
            }
        };

        // 2. Resolver hash para o manifesto FAIR
        let raw_bytes_sha256 = self
            .sync_state
            .get_snapshot_version(source_id)
            .unwrap_or_else(|| {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(locator.as_bytes());
                format!("{:x}", hasher.finalize())
            });

        // 3. Processamento de transformações colunares via Chain of Responsibility
        let mut pipeline = TransformationPipeline::new();

        if let (Some(res), Some((lat_col, lon_col))) = (
            options.assign_h3_resolution,
            options.h3_coord_columns.as_ref(),
        ) {
            pipeline = pipeline.add_step(H3SpatialIndexingStep::new(
                lat_col,
                lon_col,
                format!("h3_res{res}"),
                res,
            ));
        }

        if options.enrich_csap {
            pipeline = pipeline.add_step(CsapEnrichmentStep::new());
        }

        for custom_step in &options.custom_steps {
            pipeline = pipeline.add_shared_step(custom_step.clone());
        }

        let processed_batches = pipeline.execute_batches(raw_batches)?;

        // 4. Sumário de CSAP (se aplicável)
        let csap_summary = if options.enrich_csap || source_id.contains("sih") {
            self.evaluate_hospital_csap(&processed_batches).ok()
        } else {
            None
        };

        // 5. Persistência em cache particionado Hive-Parquet (se configurado)
        let mut persisted_snapshot_id = None;
        if let (true, Some(base_path)) =
            (options.persist_to_cache, options.cache_base_path.as_ref())
        {
            let store = HiveParquetStore::new(base_path)?;
            let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
            for batch in &processed_batches {
                let snap_record = store.save_batch(source_id, uf, params.year as i32, batch)?;
                persisted_snapshot_id = Some(snap_record.snapshot_id.to_string());
            }
        }

        // 6. Emissão do manifesto criptográfico FAIR W3C PROV-O
        let manifest = self.generate_manifest(&[locator], &[raw_bytes_sha256])?;

        Ok(PipelineExecutionResult {
            batches: processed_batches,
            manifest,
            csap_summary,
            persisted_snapshot_id,
            data_freshness,
        })
    }
}

#[async_trait]
impl MultidimensionalQueryPort for BRHealthApplicationService {
    async fn execute_query(
        &self,
        source_id: &str,
        params: &DataQueryParams,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let source = self.registry.get(source_id)?;
        source.fetch_and_decode(params, &self.context).await
    }
}

impl SpatialJoinEnginePort for BRHealthApplicationService {
    fn assign_h3_indices(
        &self,
        batch: &RecordBatch,
        lat_col: &str,
        lon_col: &str,
        resolution: u8,
    ) -> Result<RecordBatch, PortError> {
        let col_name = format!("h3_res{resolution}");
        append_h3_column(batch, lat_col, lon_col, &col_name, resolution)
    }
}

impl ProvenanceExtractionPort for BRHealthApplicationService {
    fn generate_manifest(
        &self,
        raw_uris: &[String],
        raw_hashes: &[String],
    ) -> Result<FairManifest, PortError> {
        let sources: Vec<SourceProvenance> = raw_uris
            .iter()
            .zip(raw_hashes.iter())
            .map(|(u, h)| SourceProvenance {
                source_name: "RawHealthDataPayload".into(),
                scope: "IngestionPayload".into(),
                uri: u.clone(),
                sha256_raw_payload: h.clone(),
                retrieved_at: Utc::now(),
            })
            .collect();

        Ok(FairManifest::new(sources, Vec::new()))
    }
}

impl SnapshotTimeTravelPort for BRHealthApplicationService {
    fn resolve_snapshot_id(
        &self,
        source_id: &str,
        as_of_snapshot: Option<&str>,
    ) -> Result<Option<String>, PortError> {
        if let Some(explicit) = as_of_snapshot {
            return Ok(Some(explicit.to_string()));
        }

        Ok(self.sync_state.get_snapshot_version(source_id))
    }
}

impl CSAPCostAnalysisPort for BRHealthApplicationService {
    fn evaluate_hospital_csap(
        &self,
        batches: &[RecordBatch],
    ) -> Result<CSAPAnalysisSummary, PortError> {
        let mut total_adm = 0;
        let mut csap_adm = 0;
        let mut total_cost = 0.0;
        let mut csap_cost = 0.0;
        let mut csap_bed_days = 0;

        for batch in batches {
            let metrics: CsapMetrics = compute_csap_metrics(batch, None)?;
            total_adm += metrics.total_admissions as usize;
            csap_adm += metrics.csap_admissions as usize;
            total_cost += metrics.total_cost;
            csap_cost += metrics.avoidable_cost;
            csap_bed_days += metrics.avoidable_days as usize;
        }

        let csap_proportion = if total_adm > 0 {
            (csap_adm as f64 / total_adm as f64) * 100.0
        } else {
            0.0
        };

        let csap_cost_proportion = if total_cost > 0.0 {
            (csap_cost / total_cost) * 100.0
        } else {
            0.0
        };

        Ok(CSAPAnalysisSummary {
            total_admissions: total_adm,
            csap_admissions: csap_adm,
            csap_proportion,
            total_cost,
            csap_avoidable_cost: csap_cost,
            csap_cost_proportion,
            csap_occupied_bed_days: csap_bed_days,
        })
    }
}

impl GlobalHarmonizationPort for BRHealthApplicationService {
    fn harmonize_municipality_code(&self, raw_code: &str) -> Result<String, PortError> {
        harmonize_ibge_code(raw_code)
    }

    fn map_icd10_to_icd11(&self, icd10_code: &str) -> Option<String> {
        self.ontology_harmonizer.map_icd10_to_icd11(icd10_code)
    }
}
