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

    /// Retorna a referência ao registro de fontes.
    #[must_use]
    pub fn registry(&self) -> &Arc<SourceRegistry> {
        &self.registry
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
