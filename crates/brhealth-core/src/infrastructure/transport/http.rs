// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Cliente de transporte HTTP/HTTPS streaming Tokio para consumo de dados internacionais.
//!
//! Fornece download assíncrono para endpoints da OMS (WHO GHO Athena),
//! IHME (Global Burden of Disease), Copernicus ERA5 e WorldPop.

use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::domain::ports::outbound::{PortError, TransportPort};

/// Configuração do cliente HTTP.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Timeout máximo para resposta HTTP.
    pub timeout: Duration,
    /// User-Agent enviado nas requisições.
    pub user_agent: String,
    /// Número máximo de tentativas com retries e backoff exponencial.
    pub max_retries: usize,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            user_agent: format!("BRHealth/{} (Scientific Engine)", env!("CARGO_PKG_VERSION")),
            max_retries: 3,
        }
    }
}

/// Cliente de transporte HTTP assíncrono Tokio.
#[derive(Debug, Clone)]
pub struct AsyncHttpTransport {
    config: HttpClientConfig,
    client: reqwest::Client,
}

impl AsyncHttpTransport {
    /// Cria um novo cliente com a configuração fornecida.
    pub fn new(config: HttpClientConfig) -> Result<Self, PortError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| PortError::TransportError(format!("Falha ao construir cliente HTTP: {e}")))?;

        Ok(Self { config, client })
    }

    /// Cria um novo cliente com as configurações padrão.
    pub fn new_default() -> Result<Self, PortError> {
        Self::new(HttpClientConfig::default())
    }

    /// Executa o download de um recurso HTTP calculando simultaneamente o hash SHA-256 no voo.
    pub async fn fetch_with_sha256(&self, uri: &str) -> Result<(Vec<u8>, String), PortError> {
        let bytes = self.fetch_bytes(uri).await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());
        Ok((bytes, hash))
    }
}

#[async_trait]
impl TransportPort for AsyncHttpTransport {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError> {
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(PortError::TransportError(format!(
                "URI inválida para transporte HTTP: '{uri}'"
            )));
        }

        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * (1 << attempt));
                tokio::time::sleep(backoff).await;
            }

            match self.client.get(uri).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        match response.bytes().await {
                            Ok(bytes) => return Ok(bytes.to_vec()),
                            Err(e) => {
                                last_error = Some(format!("Falha ao ler stream de resposta HTTP: {e}"));
                            }
                        }
                    } else {
                        last_error = Some(format!("Servidor HTTP retornou status {status} para '{uri}'"));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Falha na requisição HTTP para '{uri}': {e}"));
                }
            }
        }

        Err(PortError::TransportError(last_error.unwrap_or_else(|| {
            format!("Falha ao descarregar recurso HTTP '{uri}' após tentativas")
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_http_uri_rejected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let transport = AsyncHttpTransport::new_default().unwrap();
            let res = transport.fetch_bytes("ftp://ftp.datasus.gov.br/file.dbc").await;
            assert!(res.is_err());
        });
    }

    #[test]
    fn test_client_config_defaults() {
        let config = HttpClientConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!(config.user_agent.contains("BRHealth"));
    }
}

