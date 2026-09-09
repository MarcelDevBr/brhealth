// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("Recurso não encontrado: {0}")]
    ResourceNotFound(String),

    #[error("Erro de I/O na porta: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Erro de descompressão: {0}")]
    DecompressionError(String),

    #[error("Erro de decodificação tabular: {0}")]
    TabularDecodeError(String),

    #[error("Erro no cliente de transporte: {0}")]
    TransportError(String),

    #[error("Erro no armazenamento de cache: {0}")]
    CacheError(String),

    #[error("Erro de validação ou esquema: {0}")]
    ValidationError(String),

    #[error("Incompatibilidade de esquema de dados: {0}")]
    SchemaMismatch(String),

    #[error("Erro durante transformação analítica de dados: {0}")]
    TransformationError(String),

    #[error("Fonte degradada (dados stale do cache local): {0}")]
    DegradedSource(String),
}

pub trait DecompressorPort: Send + Sync {
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, PortError>;
}

#[async_trait::async_trait]
pub trait TransportPort: Send + Sync {
    async fn fetch_bytes(&self, uri: &str) -> Result<Vec<u8>, PortError>;
}

pub trait LocalCachePort: Send + Sync {
    fn exists(&self, key: &str) -> bool;
    fn read(&self, key: &str) -> Result<Vec<u8>, PortError>;
    fn write(&self, key: &str, data: &[u8]) -> Result<(), PortError>;
}

pub trait SyncStatePort: Send + Sync {
    fn get_snapshot_version(&self, source_id: &str) -> Option<String>;
    fn record_snapshot(
        &self,
        source_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(), PortError>;
}
