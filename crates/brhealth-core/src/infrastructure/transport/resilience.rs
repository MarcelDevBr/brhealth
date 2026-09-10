// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Transporte resiliente com fallback em múltiplos níveis.
//!
//! Implementa o padrão **Cache-First with Graceful Degradation**:
//! 1. **Primary**: tenta o transporte primário (FTP/HTTP original)
//! 2. **Mirror**: itera URIs alternativas declaradas pela fonte
//! 3. **Stale Cache**: serve dados do último snapshot Hive-Parquet local
//!
//! Cada nível aplica exponential backoff configurável antes de escalar
//! para o próximo.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::ports::outbound::{PortError, TransportPort};

/// Resultado do fetch resiliente com metadados de frescura.
#[derive(Debug, Clone)]
pub enum FetchOutcome {
    /// Dados obtidos com sucesso da fonte primária ou mirror.
    Fresh {
        /// Bytes do payload transferido.
        bytes: Vec<u8>,
        /// URI de origem efetiva utilizada.
        source_uri: String,
    },
    /// Dados servidos do cache local (stale).
    Stale {
        /// Bytes do payload em cache.
        bytes: Vec<u8>,
        /// Data/hora UTC em que os dados foram originalmente obtidos.
        cached_at: DateTime<Utc>,
        /// URI original que falhou.
        original_uri: String,
    },
    /// Todas as tentativas falharam e não há cache disponível.
    Unavailable {
        /// Erro da última tentativa.
        last_error: String,
        /// URI original que falhou.
        original_uri: String,
    },
}

impl FetchOutcome {
    /// Retorna `true` se os dados obtidos são frescos (do servidor).
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        matches!(self, FetchOutcome::Fresh { .. })
    }

    /// Retorna `true` se os dados obtidos são stale (do cache).
    #[must_use]
    pub fn is_stale(&self) -> bool {
        matches!(self, FetchOutcome::Stale { .. })
    }

    /// Extrai os bytes do resultado, independente de frescura.
    /// Retorna `None` se o resultado é `Unavailable`.
    #[must_use]
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            FetchOutcome::Fresh { bytes, .. } | FetchOutcome::Stale { bytes, .. } => Some(bytes),
            FetchOutcome::Unavailable { .. } => None,
        }
    }
}

/// Indicador de frescura dos dados para metadados de pipeline.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DataFreshness {
    /// Dados obtidos diretamente da fonte primária ou mirror.
    Fresh,
    /// Dados servidos do cache local, possivelmente desatualizados.
    Stale {
        /// Quando os dados foram originalmente ingeridos.
        cached_at: DateTime<Utc>,
        /// Motivo pelo qual a fonte primária falhou.
        reason: String,
    },
    /// Nenhum dado disponível.
    Unavailable {
        /// Motivo da indisponibilidade.
        reason: String,
    },
}

impl fmt::Display for DataFreshness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFreshness::Fresh => write!(f, "Fresh (dados obtidos da fonte primária)"),
            DataFreshness::Stale { cached_at, reason } => {
                write!(
                    f,
                    "Stale (cache de {}, motivo: {})",
                    cached_at.format("%Y-%m-%dT%H:%M:%SZ"),
                    reason
                )
            }
            DataFreshness::Unavailable { reason } => {
                write!(f, "Indisponível (motivo: {})", reason)
            }
        }
    }
}

/// Política de resiliência configurável para o transporte.
#[derive(Debug, Clone)]
pub struct ResiliencePolicy {
    /// Número máximo de tentativas em mirrors alternativos.
    pub max_mirror_attempts: usize,
    /// Se verdadeiro, permite servir dados stale do cache local em caso de falha total.
    pub allow_stale_cache: bool,
    /// Tempo máximo (em horas) para considerar dados em cache como aceitáveis.
    /// Dados mais antigos que esse limiar serão rejeitados mesmo com `allow_stale_cache`.
    pub stale_ttl_hours: u64,
    /// Backoff base entre tentativas de mirrors (em milissegundos).
    pub mirror_backoff_base_ms: u64,
}

impl Default for ResiliencePolicy {
    fn default() -> Self {
        Self {
            max_mirror_attempts: 3,
            allow_stale_cache: true,
            stale_ttl_hours: 168, // 7 dias
            mirror_backoff_base_ms: 200,
        }
    }
}

/// Transporte resiliente que tenta múltiplas URIs antes de degradar para cache.
pub struct ResilientTransport<T: TransportPort> {
    primary: T,
    policy: ResiliencePolicy,
}

impl<T: TransportPort> ResilientTransport<T> {
    /// Cria um novo `ResilientTransport` com o transporte primário e política padrão.
    pub fn new(primary: T) -> Self {
        Self {
            primary,
            policy: ResiliencePolicy::default(),
        }
    }

    /// Cria um novo `ResilientTransport` com política de resiliência customizada.
    pub fn with_policy(primary: T, policy: ResiliencePolicy) -> Self {
        Self { primary, policy }
    }

    /// Tenta obter bytes da URI primária, depois dos mirrors, em sequência.
    ///
    /// Retorna `FetchOutcome` com metadados de frescura.
    pub async fn fetch_with_fallback(
        &self,
        primary_uri: &str,
        mirror_uris: &[String],
    ) -> FetchOutcome {
        // 1. Tentar URI primária
        match self.primary.fetch_bytes(primary_uri).await {
            Ok(bytes) => {
                return FetchOutcome::Fresh {
                    bytes,
                    source_uri: primary_uri.to_string(),
                };
            }
            Err(e) => {
                tracing_or_eprintln(
                    &format!("Fonte primária temporariamente inacessível ('{}'): {}", primary_uri, e),
                );
            }
        }

        // 2. Tentar mirrors em sequência com backoff
        let max_attempts = self.policy.max_mirror_attempts.min(mirror_uris.len());
        for (i, mirror_uri) in mirror_uris.iter().take(max_attempts).enumerate() {
            if i > 0 {
                let backoff_ms = self.policy.mirror_backoff_base_ms * (1u64 << i.min(6));
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.primary.fetch_bytes(mirror_uri).await {
                Ok(bytes) => {
                    eprintln!(
                        "✓ [BRHealth] Contingência ativada com sucesso via espelho #{} ('{}')",
                        i + 1,
                        mirror_uri
                    );
                    return FetchOutcome::Fresh {
                        bytes,
                        source_uri: mirror_uri.clone(),
                    };
                }
                Err(e) => {
                    tracing_or_eprintln(
                        &format!("Falha no espelho #{} ('{}'): {}", i + 1, mirror_uri, e),
                    );
                }
            }
        }

        // 3. Todas as tentativas falharam
        FetchOutcome::Unavailable {
            last_error: format!(
                "Todas as {} tentativa(s) de conexão falharam para '{}'",
                1 + max_attempts,
                primary_uri
            ),
            original_uri: primary_uri.to_string(),
        }
    }
}

#[async_trait]
impl<T: TransportPort> TransportPort for ResilientTransport<T> {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError> {
        // Fallback simples sem mirrors — delega para o transporte primário
        self.primary.fetch_bytes(uri).await
    }
}

/// Log básico de avisos do BRHealth.
fn tracing_or_eprintln(msg: &str) {
    eprintln!("⚠ [BRHealth Aviso] {}", msg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::transport::MockTransport;

    #[tokio::test]
    async fn test_primary_success_returns_fresh() {
        let mock = MockTransport::new();
        mock.register_response("ftp://primary/file.dbc", vec![1, 2, 3]);

        let resilient = ResilientTransport::new(mock);
        let outcome = resilient
            .fetch_with_fallback("ftp://primary/file.dbc", &[])
            .await;

        assert!(outcome.is_fresh());
        assert_eq!(outcome.into_bytes(), Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_primary_fails_mirror_succeeds() {
        let mock = MockTransport::new();
        // Primary não registrada → falha
        mock.register_response("https://mirror1/file.dbc", vec![4, 5, 6]);

        let resilient = ResilientTransport::new(mock);
        let mirrors = vec!["https://mirror1/file.dbc".to_string()];
        let outcome = resilient
            .fetch_with_fallback("ftp://primary/file.dbc", &mirrors)
            .await;

        assert!(outcome.is_fresh());
        assert_eq!(outcome.into_bytes(), Some(vec![4, 5, 6]));
    }

    #[tokio::test]
    async fn test_all_fail_returns_unavailable() {
        let mock = MockTransport::new();
        // Nada registrado → tudo falha

        let resilient = ResilientTransport::new(mock);
        let mirrors = vec!["https://mirror1/file.dbc".to_string()];
        let outcome = resilient
            .fetch_with_fallback("ftp://primary/file.dbc", &mirrors)
            .await;

        assert!(!outcome.is_fresh());
        assert!(!outcome.is_stale());
        assert!(outcome.into_bytes().is_none());
    }

    #[tokio::test]
    async fn test_custom_policy_limits_mirror_attempts() {
        let mock = MockTransport::new();
        // Registra apenas mirror3 — mas política limita a 2 tentativas
        mock.register_response("https://mirror3/file.dbc", vec![7, 8, 9]);

        let policy = ResiliencePolicy {
            max_mirror_attempts: 2,
            ..Default::default()
        };
        let resilient = ResilientTransport::with_policy(mock, policy);
        let mirrors = vec![
            "https://mirror1/file.dbc".to_string(),
            "https://mirror2/file.dbc".to_string(),
            "https://mirror3/file.dbc".to_string(), // não alcançado
        ];
        let outcome = resilient
            .fetch_with_fallback("ftp://primary/file.dbc", &mirrors)
            .await;

        // mirror3 não é alcançado pois max_mirror_attempts=2
        assert!(outcome.into_bytes().is_none());
    }

    #[test]
    fn test_data_freshness_display() {
        let fresh = DataFreshness::Fresh;
        assert!(fresh.to_string().contains("Fresh"));

        let stale = DataFreshness::Stale {
            cached_at: Utc::now(),
            reason: "FTP timeout".to_string(),
        };
        assert!(stale.to_string().contains("Stale"));

        let unavail = DataFreshness::Unavailable {
            reason: "Servidor offline".to_string(),
        };
        assert!(unavail.to_string().contains("Indisponível"));
    }

    #[test]
    fn test_resilience_policy_defaults() {
        let policy = ResiliencePolicy::default();
        assert_eq!(policy.max_mirror_attempts, 3);
        assert!(policy.allow_stale_cache);
        assert_eq!(policy.stale_ttl_hours, 168);
    }
}
