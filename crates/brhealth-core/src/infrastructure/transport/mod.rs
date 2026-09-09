// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de transporte para download e leitura de dados brutos.

pub mod ftp;
pub mod health_check;
pub mod http;
pub mod resilience;

pub use ftp::{AsyncFtpTransport, FtpClientConfig};
pub use health_check::{HealthReport, SourceHealthChecker, SourceHealthEntry, SourceStatus};
pub use http::{AsyncHttpTransport, HttpClientConfig};
pub use resilience::{DataFreshness, FetchOutcome, ResilientTransport, ResiliencePolicy};

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::ports::outbound::{PortError, TransportPort};

/// Cliente de transporte para o sistema de arquivos local (`file://` ou caminhos diretos).
#[derive(Debug, Default, Clone)]
pub struct FileTransport;

#[async_trait]
impl TransportPort for FileTransport {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError> {
        let path_str = uri.strip_prefix("file://").unwrap_or(uri);
        let path = Path::new(path_str);
        tokio::fs::read(path).await.map_err(|e| {
            PortError::TransportError(format!("Falha ao ler arquivo local em '{uri}': {e}"))
        })
    }
}

/// Cliente de transporte simulado (Mock) em memória para testes unitários e reprodutibilidade.
#[derive(Debug, Default)]
pub struct MockTransport {
    responses: RwLock<HashMap<String, Vec<u8>>>,
}

impl MockTransport {
    /// Cria uma nova instância de MockTransport.
    #[must_use]
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
        }
    }

    /// Registra uma resposta simulada para uma URI específica.
    pub fn register_response(&self, uri: &str, data: Vec<u8>) {
        if let Ok(mut map) = self.responses.write() {
            map.insert(uri.to_string(), data);
        }
    }
}

#[async_trait]
impl TransportPort for MockTransport {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError> {
        let map = self.responses.read().map_err(|_| {
            PortError::TransportError("Falha de concorrência no MockTransport".into())
        })?;

        map.get(uri).cloned().ok_or_else(|| {
            PortError::ResourceNotFound(format!("Recurso não encontrado no MockTransport: {uri}"))
        })
    }
}
