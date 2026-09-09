// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de cache local para persistência de dados e partições analíticas.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::ports::outbound::{LocalCachePort, PortError};

/// Cache local mantido em memória RAM para testes e pipelines efêmeros.
#[derive(Debug, Default)]
pub struct MemoryCache {
    store: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryCache {
    /// Cria uma nova instância de `MemoryCache`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl LocalCachePort for MemoryCache {
    fn exists(&self, key: &str) -> bool {
        self.store
            .read()
            .map(|s| s.contains_key(key))
            .unwrap_or(false)
    }

    fn read(&self, key: &str) -> Result<Vec<u8>, PortError> {
        let store = self.store.read().map_err(|_| {
            PortError::CacheError("Falha de concorrência ao ler MemoryCache".into())
        })?;

        store.get(key).cloned().ok_or_else(|| {
            PortError::ResourceNotFound(format!("Chave '{key}' não encontrada no cache"))
        })
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), PortError> {
        let mut store = self.store.write().map_err(|_| {
            PortError::CacheError("Falha de concorrência ao escrever MemoryCache".into())
        })?;

        store.insert(key.to_string(), data.to_vec());
        Ok(())
    }
}
