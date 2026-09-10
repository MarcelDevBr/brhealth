// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de cache local para persistência de dados e partições analíticas.

use std::collections::HashMap;
use std::sync::RwLock;

use std::path::PathBuf;

use crate::domain::ports::outbound::{LocalCachePort, PortError};

/// Retorna o diretório persistente padrão de cache do BRHealth localizado na pasta HOME do usuário.
///
/// Prioridade de resolução:
/// 1. Variável de ambiente `BRHEALTH_CACHE_DIR` (se definida e não vazia).
/// 2. `$HOME/.brhealth/cache` (Linux, macOS).
/// 3. `%USERPROFILE%\.brhealth\cache` (Windows).
/// 4. Fallback `.brhealth_cache` relativo se nenhuma variável for detectada.
///
/// Isso garante que os dados em cache resistam a reinicializações da máquina,
/// impedindo a perda involuntária de dados que ocorre com pastas voláteis como `/tmp`.
#[must_use]
pub fn default_cache_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BRHEALTH_CACHE_DIR")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom.trim());
    }

    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home.trim()).join(".brhealth").join("cache");
    }

    if let Ok(userprofile) = std::env::var("USERPROFILE")
        && !userprofile.trim().is_empty()
    {
        return PathBuf::from(userprofile.trim())
            .join(".brhealth")
            .join("cache");
    }

    PathBuf::from(".brhealth_cache")
}

/// Retorna o diretório persistente padrão para dados exportados do BRHealth localizado na pasta HOME.
///
/// Exemplo: `$HOME/.brhealth/data`.
#[must_use]
pub fn default_data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BRHEALTH_DATA_DIR")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom.trim());
    }

    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home.trim()).join(".brhealth").join("data");
    }

    if let Ok(userprofile) = std::env::var("USERPROFILE")
        && !userprofile.trim().is_empty()
    {
        return PathBuf::from(userprofile.trim())
            .join(".brhealth")
            .join("data");
    }

    PathBuf::from(".brhealth_data")
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cache_dir_resolves_to_home_or_fallback() {
        let cache_path = default_cache_dir();
        // Não pode conter /tmp em sistemas Unix padrão quando HOME estiver definida
        if std::env::var("HOME").is_ok() {
            assert!(
                cache_path.to_string_lossy().contains(".brhealth"),
                "O caminho do cache deve estar associado à pasta .brhealth na HOME"
            );
            assert!(
                !cache_path.starts_with("/tmp"),
                "O caminho do cache não deve estar na pasta volátil /tmp"
            );
        }
    }

    #[test]
    fn test_default_data_dir_resolves_to_home() {
        let data_path = default_data_dir();
        if std::env::var("HOME").is_ok() {
            assert!(
                data_path.to_string_lossy().contains(".brhealth"),
                "O caminho de dados deve estar associado à pasta .brhealth na HOME"
            );
        }
    }
}
