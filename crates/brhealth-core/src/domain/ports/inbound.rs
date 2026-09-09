// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Portas de Entrada (*Inbound / Driving Ports*) da Arquitetura Hexagonal DOD.
//!
//! Define os casos de uso do núcleo analítico do BRHealth independentemente
//! de drivers de apresentação (FFI, CLI, Python bindings ou serviços web).

use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use crate::domain::ports::outbound::PortError;
use crate::domain::provenance::FairManifest;
use crate::domain::source_spi::DataQueryParams;

/// Sumário consolidado da análise epidemiológica e econômica de CSAP.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CSAPAnalysisSummary {
    /// Total de internações hospitalares analisadas ($N$).
    pub total_admissions: usize,
    /// Total de internações classificadas como causas evitáveis CSAP ($N_{csap}$).
    pub csap_admissions: usize,
    /// Proporção de internações por CSAP:
    ///
    /// $$P_{csap} = \frac{N_{csap}}{N} \times 100\%$$
    pub csap_proportion: f64,
    /// Custo financeiro direto total pago pelo SUS ($\sum \text{VAL\_TOT}$).
    pub total_cost: f64,
    /// Custo financeiro direto associado estritamente a internações evitáveis CSAP.
    pub csap_avoidable_cost: f64,
    /// Proporção de recursos financeiros consumidos por causas evitáveis:
    ///
    /// $$P_{cost} = \frac{\text{Custo}_{csap}}{\text{Custo}_{total}} \times 100\%$$
    pub csap_cost_proportion: f64,
    /// Total de diárias de internação ocupadas por causas evitáveis.
    pub csap_occupied_bed_days: usize,
}

/// Caso de Uso: Consulta Multidimensional e Ingestão de Dados de Saúde.
#[async_trait]
pub trait MultidimensionalQueryPort: Send + Sync {
    /// Executa uma consulta analítica multidimensional para a fonte e parâmetros especificados.
    async fn execute_query(
        &self,
        source_id: &str,
        params: &DataQueryParams,
    ) -> Result<Vec<RecordBatch>, PortError>;
}

/// Caso de Uso: Junções Espaciais Discretas em Memória (Uber H3 / S2).
pub trait SpatialJoinEnginePort: Send + Sync {
    /// Associa eventos pontuais a células discretas H3 e consolida tabelas por agregação geoespacial.
    fn assign_h3_indices(
        &self,
        batch: &RecordBatch,
        lat_col: &str,
        lon_col: &str,
        resolution: u8,
    ) -> Result<RecordBatch, PortError>;
}

/// Caso de Uso: Extração e Auditoria de Linhagem FAIR (W3C PROV-O).
pub trait ProvenanceExtractionPort: Send + Sync {
    /// Gera o manifesto criptográfico de proveniência para uma operação analítica executada.
    fn generate_manifest(
        &self,
        raw_uris: &[String],
        raw_hashes: &[String],
    ) -> Result<FairManifest, PortError>;
}

/// Caso de Uso: Controle de Versão e Reprodutibilidade Temporal de Snapshots (*Time-Travel*).
pub trait SnapshotTimeTravelPort: Send + Sync {
    /// Resolve o identificador de snapshot apropriado considerando o carimbo temporal de corte (`as_of_snapshot`).
    fn resolve_snapshot_id(
        &self,
        source_id: &str,
        as_of_snapshot: Option<&str>,
    ) -> Result<Option<String>, PortError>;
}

/// Caso de Uso: Economia da Saúde e Vigilância de Custos Evitáveis (CSAP).
pub trait CSAPCostAnalysisPort: Send + Sync {
    /// Analisa e classifica lotes de morbidade hospitalar (SIHSUS) segundo os 19 grupos da Portaria MS/SAS 221/2008.
    fn evaluate_hospital_csap(
        &self,
        batches: &[RecordBatch],
    ) -> Result<CSAPAnalysisSummary, PortError>;
}

/// Caso de Uso: Harmonização Federativa e Mapeamento Universal de Ontologias.
pub trait GlobalHarmonizationPort: Send + Sync {
    /// Harmoniza códigos municipais legados de 6 dígitos para o padrão canônico de 7 dígitos via Luhn Modulo 10.
    fn harmonize_municipality_code(&self, raw_code: &str) -> Result<String, PortError>;

    /// Mapeia códigos de mortalidade da CID-10 para a CID-11 da OMS.
    fn map_icd10_to_icd11(&self, icd10_code: &str) -> Option<String>;
}
