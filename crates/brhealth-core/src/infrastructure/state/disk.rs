// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Armazenamento persistente de estado de sincronização e auditoria de snapshots.
//!
//! Registra histórico imutável de snapshots com hashes SHA-256 e carimbos
//! temporais UTC em disco para auditoria científica FAIR e time-travel.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::domain::ports::outbound::{PortError, SyncStatePort};

/// Registro individual de auditoria de snapshot armazenado em disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentSnapshotEntry {
    /// Identificador da fonte de dados (ex: `"datasus-sim"`).
    pub source_id: String,
    /// Versão ou carimbo temporal do snapshot.
    pub version: String,
    /// Hash criptográfico SHA-256 do payload ingerido.
    pub sha256: String,
    /// Carimbo temporal UTC do registro.
    pub registered_at_utc: String,
}

/// Gerenciador de estado persistente em disco.
#[derive(Debug)]
pub struct DiskSyncState {
    file_path: PathBuf,
    cache: RwLock<HashMap<String, PersistentSnapshotEntry>>,
}

impl DiskSyncState {
    /// Abre ou cria uma base de estado persistente no caminho especificado.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PortError> {
        let file_path = path.as_ref().to_path_buf();
        let mut map = HashMap::new();

        if file_path.exists() {
            let mut file = File::open(&file_path).map_err(PortError::IoError)?;
            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(PortError::IoError)?;

            if !content.trim().is_empty() {
                let list: Vec<PersistentSnapshotEntry> = serde_json::from_str(&content)
                    .map_err(|e| PortError::CacheError(format!("Erro ao ler estado: {e}")))?;
                for item in list {
                    map.insert(item.source_id.clone(), item);
                }
            }
        }

        Ok(Self {
            file_path,
            cache: RwLock::new(map),
        })
    }

    fn persist_to_disk(
        &self,
        map: &HashMap<String, PersistentSnapshotEntry>,
    ) -> Result<(), PortError> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).map_err(PortError::IoError)?;
        }

        let list: Vec<&PersistentSnapshotEntry> = map.values().collect();
        let json = serde_json::to_string_pretty(&list)
            .map_err(|e| PortError::CacheError(format!("Erro ao serializar estado: {e}")))?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .map_err(PortError::IoError)?;

        file.write_all(json.as_bytes())
            .map_err(PortError::IoError)?;
        file.flush().map_err(PortError::IoError)?;

        Ok(())
    }
}

impl SyncStatePort for DiskSyncState {
    fn get_snapshot_version(&self, source_id: &str) -> Option<String> {
        self.cache
            .read()
            .ok()
            .and_then(|m| m.get(source_id).map(|e| e.version.clone()))
    }

    fn record_snapshot(
        &self,
        source_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(), PortError> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| PortError::CacheError("Falha de concorrência ao gravar estado".into()))?;

        cache.insert(
            source_id.to_string(),
            PersistentSnapshotEntry {
                source_id: source_id.to_string(),
                version: version.to_string(),
                sha256: sha256.to_string(),
                registered_at_utc: Utc::now().to_rfc3339(),
            },
        );

        self.persist_to_disk(&cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_sync_state_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let state_path = tmp.path().join("snapshots.json");

        {
            let state = DiskSyncState::open(&state_path).unwrap();
            assert_eq!(state.get_snapshot_version("datasus-sim"), None);

            state
                .record_snapshot("datasus-sim", "20240315", "abcdef123456")
                .unwrap();
            assert_eq!(
                state.get_snapshot_version("datasus-sim"),
                Some("20240315".to_string())
            );
        }

        // Reabertura para verificar persistência em disco
        {
            let reopened = DiskSyncState::open(&state_path).unwrap();
            assert_eq!(
                reopened.get_snapshot_version("datasus-sim"),
                Some("20240315".to_string())
            );
        }
    }
}
