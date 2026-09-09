// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Armazenamento Relacional de Snapshots e Auditoria Criptográfica em SQLite.
//!
//! Implementa o container `sqlite_sync` especificado no C4 Container do SDD,
//! garantindo persistência transacional com índices B-Tree sobre hashes SHA-256,
//! identificadores de fontes de dados e carimbos temporais UTC para time-travel e FAIR.

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{Connection, params};

use crate::domain::ports::outbound::{PortError, SyncStatePort};

/// Gerenciador relacional de estado e snapshots baseado em banco embutido SQLite.
pub struct SqliteSyncState {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for SqliteSyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteSyncState").finish_non_exhaustive()
    }
}

impl SqliteSyncState {
    /// Abre ou cria um banco relacional SQLite no caminho especificado.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PortError> {
        let conn = Connection::open(path)
            .map_err(|e| PortError::CacheError(format!("Erro ao abrir SQLite: {e}")))?;
        Self::init_schema(conn)
    }

    /// Cria um banco SQLite transitório puramente em memória (ideal para testes).
    pub fn open_in_memory() -> Result<Self, PortError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| PortError::CacheError(format!("Erro ao criar SQLite in-memory: {e}")))?;
        Self::init_schema(conn)
    }

    fn init_schema(conn: Connection) -> Result<Self, PortError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS dataset_snapshots (
                source_id TEXT NOT NULL,
                version TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                registered_at_utc TEXT NOT NULL,
                PRIMARY KEY (source_id, version)
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_sha ON dataset_snapshots(sha256);
            "#,
        )
        .map_err(|e| PortError::CacheError(format!("Erro ao inicializar schema SQLite: {e}")))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl SyncStatePort for SqliteSyncState {
    fn get_snapshot_version(&self, source_id: &str) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT version FROM dataset_snapshots WHERE source_id = ?1 ORDER BY registered_at_utc DESC LIMIT 1",
            )
            .ok()?;
        let mut rows = stmt.query(params![source_id]).ok()?;
        if let Ok(Some(row)) = rows.next() {
            row.get(0).ok()
        } else {
            None
        }
    }

    fn record_snapshot(
        &self,
        source_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(), PortError> {
        self.register_snapshot(source_id, version, sha256)
    }
}

impl SqliteSyncState {
    /// Obtém o hash SHA-256 gravado para um snapshot específico de uma fonte.
    pub fn get_snapshot_hash(&self, source_id: &str, version: &str) -> Result<Option<String>, PortError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| PortError::CacheError("Lock poisoned no SqliteSyncState".into()))?;

        let mut stmt = conn
            .prepare_cached(
                "SELECT sha256 FROM dataset_snapshots WHERE source_id = ?1 AND version = ?2",
            )
            .map_err(|e| PortError::CacheError(format!("Erro ao preparar consulta SQLite: {e}")))?;

        let mut rows = stmt
            .query(params![source_id, version])
            .map_err(|e| PortError::CacheError(format!("Erro ao executar consulta SQLite: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| PortError::CacheError(format!("Erro ao iterar linhas SQLite: {e}")))?
        {
            let sha: String = row
                .get(0)
                .map_err(|e| PortError::CacheError(format!("Erro ao extrair SHA SQLite: {e}")))?;
            Ok(Some(sha))
        } else {
            Ok(None)
        }
    }

    /// Registra um novo snapshot com controle transacional no SQLite.
    pub fn register_snapshot(
        &self,
        source_id: &str,
        version: &str,
        sha256: &str,
    ) -> Result<(), PortError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| PortError::CacheError("Lock poisoned no SqliteSyncState".into()))?;

        let now_utc = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO dataset_snapshots (source_id, version, sha256, registered_at_utc)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(source_id, version) DO UPDATE SET
                sha256 = excluded.sha256,
                registered_at_utc = excluded.registered_at_utc
            "#,
            params![source_id, version, sha256, now_utc],
        )
        .map_err(|e| PortError::CacheError(format!("Erro ao registrar snapshot SQLite: {e}")))?;

        Ok(())
    }

    /// Lista todas as versões de snapshots registradas para uma fonte de dados.
    pub fn list_snapshots(&self, source_id: &str) -> Result<Vec<String>, PortError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| PortError::CacheError("Lock poisoned no SqliteSyncState".into()))?;

        let mut stmt = conn
            .prepare_cached(
                "SELECT version FROM dataset_snapshots WHERE source_id = ?1 ORDER BY registered_at_utc ASC",
            )
            .map_err(|e| PortError::CacheError(format!("Erro ao preparar listagem SQLite: {e}")))?;

        let rows = stmt
            .query_map(params![source_id], |row| row.get(0))
            .map_err(|e| PortError::CacheError(format!("Erro ao consultar snapshots SQLite: {e}")))?;

        let mut versions = Vec::new();
        for r in rows {
            let ver: String =
                r.map_err(|e| PortError::CacheError(format!("Erro ao ler versão SQLite: {e}")))?;
            versions.push(ver);
        }

        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_sync_state_in_memory() {
        let state = SqliteSyncState::open_in_memory().unwrap();

        // 1. Consulta inicial vazia
        assert!(
            state
                .get_snapshot_hash("datasus-sim", "2024-SP")
                .unwrap()
                .is_none()
        );

        // 2. Registro de snapshot
        state
            .register_snapshot(
                "datasus-sim",
                "2024-SP",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            )
            .unwrap();

        // 3. Consulta do hash
        let hash = state
            .get_snapshot_hash("datasus-sim", "2024-SP")
            .unwrap();
        assert_eq!(
            hash,
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
        );

        // 4. Lista de snapshots
        state
            .register_snapshot("datasus-sim", "2023-SP", "hash2023")
            .unwrap();
        let list = state.list_snapshots("datasus-sim").unwrap();
        assert_eq!(list.len(), 2);
    }
}
