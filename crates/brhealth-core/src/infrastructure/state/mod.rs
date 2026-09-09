// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptadores de estado de sincronização e auditoria de snapshots.

pub mod disk;

pub use disk::{DiskSyncState, PersistentSnapshotEntry};

use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::ports::outbound::{PortError, SyncStatePort};

/// Registro de snapshot armazenado.
#[derive(Debug, Clone)]
pub struct SnapshotRecord {
    pub source_id: String,
    pub version: String,
    pub sha256: String,
}

/// Gerenciador de estado de sincronização em memória para testes e execução volátil.
#[derive(Debug, Default)]
pub struct MemorySyncState {
    records: RwLock<HashMap<String, SnapshotRecord>>,
}

impl MemorySyncState {
    /// Cria uma nova instância de `MemorySyncState`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }
}

impl SyncStatePort for MemorySyncState {
    fn get_snapshot_version(&self, source_id: &str) -> Option<String> {
        self.records
            .read()
            .ok()
            .and_then(|r| r.get(source_id).map(|rec| rec.version.clone()))
    }

    fn record_snapshot(
        &self,
        source_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(), PortError> {
        let mut records = self
            .records
            .write()
            .map_err(|_| PortError::CacheError("Falha de concorrência ao gravar estado".into()))?;

        records.insert(
            source_id.to_string(),
            SnapshotRecord {
                source_id: source_id.to_string(),
                version: version.to_string(),
                sha256: sha256.to_string(),
            },
        );

        Ok(())
    }
}
