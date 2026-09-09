// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Cliente assíncrono de transporte FTP para download resiliente de bases do DATASUS.
//!
//! Implementa reconexões automáticas com *exponential backoff*, validação de
//! *stream hashing* SHA-256 e suporte a comandos do protocolo FTP (RFC 959).

use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::domain::ports::outbound::{PortError, TransportPort};

/// Configurações de resiliência e rede para o cliente FTP do DATASUS.
#[derive(Debug, Clone)]
pub struct FtpClientConfig {
    /// Tempo limite de conexão e resposta.
    pub timeout: Duration,
    /// Número máximo de tentativas de reconexão (*retries*).
    pub max_retries: usize,
    /// Host padrão caso não especificado na URI.
    pub default_host: String,
    /// Porta FTP de controle (padrão 21).
    pub port: u16,
}

impl Default for FtpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            default_host: "ftp.datasus.gov.br".to_string(),
            port: 21,
        }
    }
}

/// Cliente FTP assíncrono Tokio com foco em resiliência governamental.
#[derive(Debug, Clone)]
pub struct AsyncFtpTransport {
    config: FtpClientConfig,
}

impl AsyncFtpTransport {
    /// Cria uma nova instância com a configuração informada.
    #[must_use]
    pub fn new(config: FtpClientConfig) -> Self {
        Self { config }
    }

    /// Cria uma nova instância com as configurações padrão para o DATASUS.
    #[must_use]
    pub fn new_datasus() -> Self {
        Self::new(FtpClientConfig::default())
    }

    /// Executa o download de um arquivo com cálculo simultâneo de hash SHA-256.
    pub async fn fetch_with_sha256(&self, uri: &str) -> Result<(Vec<u8>, String), PortError> {
        let bytes = self.fetch_bytes(uri).await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());
        Ok((bytes, hash))
    }

    /// Extrai host e caminho a partir de uma URI `ftp://`.
    fn parse_ftp_uri<'a>(&'a self, uri: &'a str) -> (&'a str, &'a str) {
        let trimmed = uri.strip_prefix("ftp://").unwrap_or(uri);
        match trimmed.find('/') {
            Some(idx) => (&trimmed[..idx], &trimmed[idx..]),
            None => (self.config.default_host.as_str(), trimmed),
        }
    }

    /// Lê uma linha ou bloco de resposta do canal de controle FTP (RFC 959).
    async fn read_ftp_response(stream: &mut TcpStream) -> Result<(u16, String), PortError> {
        let mut response_text = String::new();
        loop {
            let mut line = Vec::new();
            let mut byte_buf = [0u8; 1];
            loop {
                let n = stream.read(&mut byte_buf).await.map_err(|e| {
                    PortError::TransportError(format!("Erro ao ler canal de controle FTP: {e}"))
                })?;
                if n == 0 {
                    break;
                }
                line.push(byte_buf[0]);
                if line.ends_with(b"\n") {
                    break;
                }
            }

            if line.is_empty() {
                break;
            }

            let line_str = String::from_utf8_lossy(&line).to_string();
            response_text.push_str(&line_str);

            // Código de 3 dígitos no início da linha
            if line_str.len() >= 4 {
                let code_digits = &line_str[0..3];
                let separator = line_str.chars().nth(3).unwrap_or(' ');
                if let Ok(code) = code_digits.parse::<u16>() {
                    // Se o separador for espaço (' '), encerra a resposta (não é resposta multilinha com '-')
                    if separator == ' ' || separator == '\r' || separator == '\n' {
                        return Ok((code, response_text));
                    }
                }
            }
        }

        // Tentar extrair código da resposta acumulada
        let code = response_text
            .get(0..3)
            .and_then(|c| c.parse::<u16>().ok())
            .unwrap_or(0);
        Ok((code, response_text))
    }

    /// Faz o parse da resposta PASV (código 227) e extrai a porta de dados.
    fn parse_pasv_response(resp: &str) -> Result<u16, PortError> {
        let start = resp.find('(').ok_or_else(|| {
            PortError::TransportError(format!("Resposta PASV malformada (sem parênteses): {resp}"))
        })?;
        let end = resp.find(')').ok_or_else(|| {
            PortError::TransportError(format!("Resposta PASV malformada (sem fecha parêntese): {resp}"))
        })?;

        let numbers: Vec<u16> = resp[start + 1..end]
            .split(',')
            .filter_map(|s| s.trim().parse::<u16>().ok())
            .collect();

        if numbers.len() < 6 {
            return Err(PortError::TransportError(format!(
                "Resposta PASV contém menos de 6 números: {resp}"
            )));
        }

        let port = (numbers[4] << 8) | numbers[5];
        Ok(port)
    }
}

#[async_trait]
impl TransportPort for AsyncFtpTransport {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError> {
        let (host, path) = self.parse_ftp_uri(uri);
        let effective_host = if host.is_empty() {
            &self.config.default_host
        } else {
            host
        };

        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * (1 << attempt));
                tokio::time::sleep(backoff).await;
            }

            let addr = format!("{effective_host}:{}", self.config.port);

            let connect_fut = tokio::time::timeout(self.config.timeout, async {
                let mut ctrl_stream = TcpStream::connect(&addr).await.map_err(|e| {
                    PortError::TransportError(format!("Falha ao conectar canal de controle {addr}: {e}"))
                })?;

                // 1. Banner inicial (220)
                let (code, banner) = Self::read_ftp_response(&mut ctrl_stream).await?;
                if code != 220 {
                    return Err(PortError::TransportError(format!(
                        "Servidor FTP rejeitou conexão com código {code}: {banner}"
                    )));
                }

                // 2. Autenticação anônima (USER anonymous)
                ctrl_stream.write_all(b"USER anonymous\r\n").await?;
                let (code, resp) = Self::read_ftp_response(&mut ctrl_stream).await?;
                if code == 331 {
                    ctrl_stream
                        .write_all(b"PASS brhealth@healthanalytics.org\r\n")
                        .await?;
                    let (code_pass, resp_pass) =
                        Self::read_ftp_response(&mut ctrl_stream).await?;
                    if code_pass != 230 {
                        return Err(PortError::TransportError(format!(
                            "Falha na autenticação FTP (PASS) código {code_pass}: {resp_pass}"
                        )));
                    }
                } else if code != 230 {
                    return Err(PortError::TransportError(format!(
                        "Falha na autenticação FTP (USER) código {code}: {resp}"
                    )));
                }

                // 3. Modo binário (TYPE I)
                ctrl_stream.write_all(b"TYPE I\r\n").await?;
                let (code, resp) = Self::read_ftp_response(&mut ctrl_stream).await?;
                if code != 200 {
                    return Err(PortError::TransportError(format!(
                        "Falha ao definir modo binário (TYPE I) código {code}: {resp}"
                    )));
                }

                // 4. Modo passivo (PASV)
                ctrl_stream.write_all(b"PASV\r\n").await?;
                let (code, resp_pasv) = Self::read_ftp_response(&mut ctrl_stream).await?;
                if code != 227 {
                    return Err(PortError::TransportError(format!(
                        "Servidor FTP não aceitou comando PASV (código {code}): {resp_pasv}"
                    )));
                }

                let data_port = Self::parse_pasv_response(&resp_pasv)?;
                let data_addr = format!("{effective_host}:{data_port}");

                // 5. Conecta o canal de dados
                let mut data_stream = TcpStream::connect(&data_addr).await.map_err(|e| {
                    PortError::TransportError(format!(
                        "Falha ao conectar canal de dados FTP em {data_addr}: {e}"
                    ))
                })?;

                // 6. Solicita o arquivo (RETR)
                let clean_path = path.trim_start_matches('/');
                let retr_cmd = format!("RETR /{clean_path}\r\n");
                ctrl_stream.write_all(retr_cmd.as_bytes()).await?;

                let (code, resp_retr) = Self::read_ftp_response(&mut ctrl_stream).await?;
                if code != 150 && code != 125 {
                    return Err(PortError::TransportError(format!(
                        "Servidor FTP rejeitou comando RETR /{clean_path} (código {code}): {resp_retr}"
                    )));
                }

                // 7. Streaming dos bytes de dados
                let mut output = Vec::new();
                data_stream.read_to_end(&mut output).await.map_err(|e| {
                    PortError::TransportError(format!("Erro durante download de dados FTP: {e}"))
                })?;

                // Fecha data_stream explicitamente
                drop(data_stream);

                // 8. Confirmação de conclusão de transferência (226)
                let (code_complete, resp_complete) =
                    Self::read_ftp_response(&mut ctrl_stream).await?;
                if code_complete != 226 && code_complete != 250 {
                    return Err(PortError::TransportError(format!(
                        "Transferência FTP não finalizou com sucesso (código {code_complete}): {resp_complete}"
                    )));
                }

                Ok(output)
            });

            match connect_fut.await {
                Ok(Ok(bytes)) => return Ok(bytes),
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    last_error = Some(format!("Timeout de conexão com {addr}"));
                }
            }
        }

        Err(PortError::TransportError(last_error.unwrap_or_else(|| {
            format!("Falha de conexão FTP com '{uri}'")
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pasv_response() {
        let resp = "227 Entering Passive Mode (192,168,1,50,195,80).";
        let port = AsyncFtpTransport::parse_pasv_response(resp).unwrap();
        assert_eq!(port, (195 << 8) | 80);
        assert_eq!(port, 50000);
    }

    #[test]
    fn test_parse_ftp_uri() {
        let client = AsyncFtpTransport::new_datasus();
        let (host, path) =
            client.parse_ftp_uri("ftp://ftp.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DOAC2022.dbc");
        assert_eq!(host, "ftp.datasus.gov.br");
        assert_eq!(path, "/dissemin/publicos/SIM/CID10/DORES/DOAC2022.dbc");
    }
}
