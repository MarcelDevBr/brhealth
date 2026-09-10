// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Verificador de saúde assíncrono para fontes de dados.
//!
//! Executa probes leves (HEAD HTTP / conexão TCP FTP) em cada fonte
//! registrada e emite um relatório de disponibilidade antes da
//! ingestão completa.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use crate::domain::source_spi::{DataQueryParams, GeographicScope, HealthDataSourceSPI};

/// Status de saúde de uma fonte de dados individual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceStatus {
    /// Fonte respondeu com sucesso dentro do tempo limite.
    Healthy,
    /// Fonte respondeu, mas com latência elevada ou código de resposta parcial.
    Degraded {
        /// Motivo da degradação observada.
        reason: String,
    },
    /// Fonte não respondeu ou retornou erro fatal.
    Offline {
        /// Motivo da indisponibilidade.
        reason: String,
    },
    /// Probe não executado (fonte ignorada ou não suportada).
    Skipped {
        /// Motivo pelo qual o probe foi ignorado.
        reason: String,
    },
}

/// Resultado individual de um probe de saúde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealthEntry {
    /// Identificador da fonte (ex: `"datasus.sim"`).
    pub source_id: String,
    /// Nome amigável da fonte para exibição.
    pub display_name: String,
    /// Status de saúde verificado.
    pub status: SourceStatus,
    /// Latência medida do probe (quando aplicável).
    pub latency_ms: Option<u64>,
    /// Carimbo temporal UTC do probe.
    pub checked_at: DateTime<Utc>,
}

/// Relatório consolidado de saúde de todas as fontes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Lista de resultados individuais por fonte.
    pub entries: Vec<SourceHealthEntry>,
    /// Carimbo temporal UTC do início da verificação.
    pub started_at: DateTime<Utc>,
    /// Duração total da verificação.
    pub total_duration_ms: u64,
}

impl HealthReport {
    /// Retorna o número de fontes saudáveis.
    #[must_use]
    pub fn healthy_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status == SourceStatus::Healthy)
            .count()
    }

    /// Retorna o número de fontes offline.
    #[must_use]
    pub fn offline_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, SourceStatus::Offline { .. }))
            .count()
    }

    /// Retorna o número total de fontes verificadas (excluindo skipped).
    #[must_use]
    pub fn checked_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !matches!(e.status, SourceStatus::Skipped { .. }))
            .count()
    }

    /// Resumo textual amigável do relatório.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Health Check: {}/{} saudáveis, {} degradadas, {} offline ({}ms total)",
            self.healthy_count(),
            self.checked_count(),
            self.entries
                .iter()
                .filter(|e| matches!(e.status, SourceStatus::Degraded { .. }))
                .count(),
            self.offline_count(),
            self.total_duration_ms,
        )
    }
}

/// Verificador de saúde assíncrono para fontes de dados do BRHealth.
pub struct SourceHealthChecker {
    /// Timeout máximo para cada probe individual.
    probe_timeout: Duration,
    /// Limiar de latência (ms) acima do qual a fonte é considerada degradada.
    degraded_threshold_ms: u64,
}

impl Default for SourceHealthChecker {
    fn default() -> Self {
        Self {
            probe_timeout: Duration::from_secs(10),
            degraded_threshold_ms: 5000,
        }
    }
}

impl SourceHealthChecker {
    /// Cria um novo `SourceHealthChecker` com configuração padrão.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria um `SourceHealthChecker` com parâmetros customizados.
    #[must_use]
    pub fn with_config(probe_timeout: Duration, degraded_threshold_ms: u64) -> Self {
        Self {
            probe_timeout,
            degraded_threshold_ms,
        }
    }

    /// Executa probes de saúde em todas as fontes do registro fornecido.
    ///
    /// Cada fonte é verificada com um probe leve:
    /// - FTP: conexão TCP na porta 21
    /// - HTTP/HTTPS: requisição HEAD
    /// - Fontes sem autenticação ou locais: marcadas como `Skipped`
    pub async fn check_all(&self, sources: &[Arc<dyn HealthDataSourceSPI>]) -> HealthReport {
        let started_at = Utc::now();
        let start_instant = Instant::now();

        let mut entries = Vec::with_capacity(sources.len());

        // Criar parâmetros mínimos de probe (ano corrente, escopo nacional)
        let probe_params = DataQueryParams {
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            jurisdiction_code: Some("SP".to_string()),
            year: 2023,
            month: None,
            extra_filters: HashMap::new(),
            as_of_snapshot: None,
        };

        for source in sources {
            let meta = source.metadata();
            let source_id = meta.id.to_string();
            let display_name = meta.display_name.to_string();

            // Resolver a URI para probe
            let probe_uri = match source.resolve_locator(&probe_params) {
                Ok(uri) => uri,
                Err(e) => {
                    entries.push(SourceHealthEntry {
                        source_id,
                        display_name,
                        status: SourceStatus::Skipped {
                            reason: format!("Erro ao resolver locator: {e}"),
                        },
                        latency_ms: None,
                        checked_at: Utc::now(),
                    });
                    continue;
                }
            };

            let entry = self.probe_uri(&source_id, &display_name, &probe_uri).await;
            entries.push(entry);
        }

        let total_duration_ms = start_instant.elapsed().as_millis() as u64;

        HealthReport {
            entries,
            started_at,
            total_duration_ms,
        }
    }

    /// Executa probe individual em uma URI.
    async fn probe_uri(&self, source_id: &str, display_name: &str, uri: &str) -> SourceHealthEntry {
        let start = Instant::now();

        let status = if uri.starts_with("ftp://") {
            self.probe_ftp(uri).await
        } else if uri.starts_with("http://") || uri.starts_with("https://") {
            self.probe_http(uri).await
        } else {
            SourceStatus::Skipped {
                reason: format!("Protocolo não suportado para probe: {uri}"),
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        // Reclassificar como degraded se latência exceder o limiar
        let final_status = match &status {
            SourceStatus::Healthy if latency_ms > self.degraded_threshold_ms => {
                SourceStatus::Degraded {
                    reason: format!("Latência elevada: {latency_ms}ms"),
                }
            }
            other => other.clone(),
        };

        SourceHealthEntry {
            source_id: source_id.to_string(),
            display_name: display_name.to_string(),
            status: final_status,
            latency_ms: Some(latency_ms),
            checked_at: Utc::now(),
        }
    }

    /// Probe FTP: tenta conexão TCP na porta 21 do host.
    async fn probe_ftp(&self, uri: &str) -> SourceStatus {
        let trimmed = uri.strip_prefix("ftp://").unwrap_or(uri);
        let host = match trimmed.find('/') {
            Some(idx) => &trimmed[..idx],
            None => trimmed,
        };

        let addr = format!("{host}:21");

        match tokio::time::timeout(self.probe_timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => SourceStatus::Healthy,
            Ok(Err(e)) => SourceStatus::Offline {
                reason: format!("Conexão FTP recusada em {addr}: {e}"),
            },
            Err(_) => SourceStatus::Offline {
                reason: format!("Timeout de conexão FTP em {addr}"),
            },
        }
    }

    /// Probe HTTP: requisição HEAD com timeout.
    async fn probe_http(&self, uri: &str) -> SourceStatus {
        let client = match reqwest::Client::builder()
            .timeout(self.probe_timeout)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return SourceStatus::Offline {
                    reason: format!("Falha ao construir cliente HTTP para probe: {e}"),
                };
            }
        };

        match client.head(uri).send().await {
            Ok(response) => {
                let status_code = response.status();
                if status_code.is_success() || status_code.as_u16() == 405 {
                    // 405 Method Not Allowed é aceitável (servidor existe mas rejeita HEAD)
                    SourceStatus::Healthy
                } else if status_code.is_server_error() {
                    SourceStatus::Degraded {
                        reason: format!("Servidor retornou {status_code}"),
                    }
                } else {
                    // 4xx pode significar que o endpoint existe mas requer autenticação
                    SourceStatus::Healthy
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    SourceStatus::Offline {
                        reason: format!("Timeout HTTP em {uri}"),
                    }
                } else if e.is_connect() {
                    SourceStatus::Offline {
                        reason: format!("Conexão HTTP recusada em {uri}: {e}"),
                    }
                } else {
                    SourceStatus::Offline {
                        reason: format!("Erro HTTP em {uri}: {e}"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_report_counts() {
        let entries = vec![
            SourceHealthEntry {
                source_id: "src1".into(),
                display_name: "Source 1".into(),
                status: SourceStatus::Healthy,
                latency_ms: Some(50),
                checked_at: Utc::now(),
            },
            SourceHealthEntry {
                source_id: "src2".into(),
                display_name: "Source 2".into(),
                status: SourceStatus::Offline {
                    reason: "Timeout".into(),
                },
                latency_ms: Some(10000),
                checked_at: Utc::now(),
            },
            SourceHealthEntry {
                source_id: "src3".into(),
                display_name: "Source 3".into(),
                status: SourceStatus::Degraded {
                    reason: "Lento".into(),
                },
                latency_ms: Some(6000),
                checked_at: Utc::now(),
            },
            SourceHealthEntry {
                source_id: "src4".into(),
                display_name: "Source 4".into(),
                status: SourceStatus::Skipped {
                    reason: "Local".into(),
                },
                latency_ms: None,
                checked_at: Utc::now(),
            },
        ];

        let report = HealthReport {
            entries,
            started_at: Utc::now(),
            total_duration_ms: 16050,
        };

        assert_eq!(report.healthy_count(), 1);
        assert_eq!(report.offline_count(), 1);
        assert_eq!(report.checked_count(), 3); // exclui skipped
        assert!(report.summary().contains("1/3 saudáveis"));
    }

    #[test]
    fn test_source_status_serialization() {
        let healthy = SourceStatus::Healthy;
        let json = serde_json::to_string(&healthy).unwrap();
        let deserialized: SourceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SourceStatus::Healthy);

        let offline = SourceStatus::Offline {
            reason: "Connection refused".into(),
        };
        let json = serde_json::to_string(&offline).unwrap();
        let deserialized: SourceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, offline);
    }

    #[test]
    fn test_health_checker_default_config() {
        let checker = SourceHealthChecker::new();
        assert_eq!(checker.probe_timeout, Duration::from_secs(10));
        assert_eq!(checker.degraded_threshold_ms, 5000);
    }

    #[test]
    fn test_health_checker_custom_config() {
        let checker = SourceHealthChecker::with_config(Duration::from_secs(5), 2000);
        assert_eq!(checker.probe_timeout, Duration::from_secs(5));
        assert_eq!(checker.degraded_threshold_ms, 2000);
    }
}
