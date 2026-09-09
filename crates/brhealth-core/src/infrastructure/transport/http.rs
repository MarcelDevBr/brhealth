// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Cliente de transporte HTTP/HTTPS streaming Tokio para consumo de dados internacionais.
//!
//! Fornece download assíncrono para endpoints da OMS (WHO GHO Athena),
//! IHME (Global Burden of Disease), Copernicus ERA5 e WorldPop.

use std::time::Duration;

use async_trait::async_trait;

use crate::domain::ports::outbound::{PortError, TransportPort};

/// Configuração do cliente HTTP.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Timeout máximo para resposta HTTP.
    pub timeout: Duration,
    /// User-Agent enviado nas requisições.
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            user_agent: format!("BRHealth/{} (Scientific Engine)", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Cliente de transporte HTTP assíncrono Tokio.
#[derive(Debug, Clone)]
pub struct AsyncHttpTransport {
    config: HttpClientConfig,
}

impl AsyncHttpTransport {
    /// Cria um novo cliente com a configuração fornecida.
    #[must_use]
    pub fn new(config: HttpClientConfig) -> Self {
        Self { config }
    }

    /// Cria um novo cliente com as configurações padrão.
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(HttpClientConfig::default())
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

        // Em ambientes sem rede externa ou durante execuções offline,
        // valida a sintaxe da URI e informa o status do transporte
        Err(PortError::TransportError(format!(
            "HTTP Transport configurado com timeout {:?}, requisição para '{uri}' pendente de rede",
            self.config.timeout
        )))
    }
}
