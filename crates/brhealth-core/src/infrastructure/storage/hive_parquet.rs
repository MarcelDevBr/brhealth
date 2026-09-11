// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Armazenamento colunar particionado no formato Apache Hive-Parquet com suporte a Time-Travel.
//!
//! Implementa versionamento imutável de dados analíticos com linhagem auditável,
//! permitindo leituras históricas determinísticas (`as_of_snapshot`).

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ports::outbound::PortError;
use crate::domain::provenance::compute_sha256;

/// Registro de snapshot particionado para time-travel e reprodutibilidade científica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotRecord {
    /// Identificador universal único do snapshot.
    pub snapshot_id: Uuid,
    /// Identificador do conjunto de dados (ex: `"datasus.sih"`, `"datasus.sim"`).
    pub dataset_id: String,
    /// Unidade Federativa da partição (ex: `"AC"`, `"SP"`).
    pub uf: String,
    /// Ano de competência da partição.
    pub year: i32,
    /// Carimbo temporal UTC no momento da gravação.
    pub created_at: DateTime<Utc>,
    /// Quantidade de registros tabulares no lote.
    pub num_rows: usize,
    /// Caminho relativo do arquivo Parquet.
    pub file_path: String,
    /// Hash criptográfico SHA-256 do arquivo Parquet gerado.
    pub sha256_parquet: String,
}

/// Catálogo de snapshots persistido no diretório `_metadata/snapshots.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotCatalog {
    pub snapshots: Vec<SnapshotRecord>,
}

/// Adaptador de infraestrutura para armazenamento colunar Hive-Parquet.
pub struct HiveParquetStore {
    base_dir: PathBuf,
    lock: RwLock<()>,
}

impl HiveParquetStore {
    /// Cria uma nova instância apontando para o diretório base do data lakehouse local.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, PortError> {
        let base_path = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_path).map_err(PortError::IoError)?;
        Ok(Self {
            base_dir: base_path,
            lock: RwLock::new(()),
        })
    }

    /// Grava um `RecordBatch` Apache Arrow em partição Hive com novo snapshot imutável.
    ///
    /// O caminho gerado obedece à convenção Hive:
    /// `{base_dir}/{dataset_id}/uf={uf}/year={year}/snapshot={snapshot_id}/data.parquet`
    pub fn save_batch(
        &self,
        dataset_id: &str,
        uf: &str,
        year: i32,
        batch: &RecordBatch,
    ) -> Result<SnapshotRecord, PortError> {
        let _guard = self.lock.write().map_err(|_| {
            PortError::CacheError("Falha de concorrência no HiveParquetStore".into())
        })?;

        let snapshot_id = Uuid::new_v4();
        let relative_dir = PathBuf::from(dataset_id)
            .join(format!("uf={}", uf.to_uppercase()))
            .join(format!("year={year}"))
            .join(format!("snapshot={snapshot_id}"));

        let full_dir = self.base_dir.join(&relative_dir);
        fs::create_dir_all(&full_dir).map_err(PortError::IoError)?;

        let relative_file = relative_dir.join("data.parquet");
        let full_file = self.base_dir.join(&relative_file);

        // Escrever arquivo Parquet com Apache ArrowWriter
        {
            let file = File::create(&full_file).map_err(PortError::IoError)?;
            let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
                .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
            writer
                .write(batch)
                .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
            writer
                .close()
                .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
        }

        // Calcular SHA-256 do arquivo gravado
        let file_bytes = fs::read(&full_file).map_err(PortError::IoError)?;
        let sha256_parquet = compute_sha256(&file_bytes);

        let record = SnapshotRecord {
            snapshot_id,
            dataset_id: dataset_id.to_string(),
            uf: uf.to_uppercase(),
            year,
            created_at: Utc::now(),
            num_rows: batch.num_rows(),
            file_path: relative_file.to_string_lossy().to_string(),
            sha256_parquet,
        };

        // Atualizar catálogo de snapshots
        self.append_to_catalog(dataset_id, &record)?;

        Ok(record)
    }

    /// Lê diretamente o `RecordBatch` associado a um `snapshot_id` específico.
    pub fn read_snapshot(
        &self,
        dataset_id: &str,
        snapshot_id: Uuid,
    ) -> Result<RecordBatch, PortError> {
        let catalog = self.load_catalog(dataset_id)?;
        let record = catalog
            .snapshots
            .iter()
            .find(|s| s.snapshot_id == snapshot_id)
            .ok_or_else(|| {
                PortError::ResourceNotFound(format!("Snapshot {snapshot_id} não encontrado"))
            })?;

        let full_file = self.base_dir.join(&record.file_path);
        let file = File::open(&full_file).map_err(PortError::IoError)?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
        let mut reader = builder
            .build()
            .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;

        // Consumir o primeiro RecordBatch gravado
        let batch_opt = reader
            .next()
            .transpose()
            .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;

        batch_opt.ok_or_else(|| {
            PortError::TabularDecodeError("Arquivo Parquet vazio ou corrompido".into())
        })
    }

    /// Lê a versão mais recente dos dados que existia até o momento temporal `as_of` (Time-Travel).
    pub fn read_as_of(
        &self,
        dataset_id: &str,
        uf: &str,
        year: i32,
        as_of: DateTime<Utc>,
    ) -> Result<Option<RecordBatch>, PortError> {
        let catalog = self.load_catalog(dataset_id)?;
        let upper_uf = uf.to_uppercase();

        // Encontrar o snapshot mais recente que foi criado antes ou exatamente em `as_of`
        let matching_snapshot = catalog
            .snapshots
            .iter()
            .filter(|s| s.uf == upper_uf && s.year == year && s.created_at <= as_of)
            .max_by_key(|s| s.created_at);

        match matching_snapshot {
            Some(record) => {
                let batch = self.read_snapshot(dataset_id, record.snapshot_id)?;
                Ok(Some(batch))
            }
            None => Ok(None),
        }
    }

    /// Lista todos os snapshots registrados para o conjunto de dados especificado.
    pub fn list_snapshots(&self, dataset_id: &str) -> Result<Vec<SnapshotRecord>, PortError> {
        let catalog = self.load_catalog(dataset_id)?;
        Ok(catalog.snapshots)
    }

    /// Limpa o cache Hive-Parquet. Se `dataset_id` for fornecido, limpa apenas a pasta
    /// e o catálogo daquele conjunto de dados; caso contrário, limpa todo o diretório base.
    /// Retorna o número de diretórios/arquivos removidos.
    pub fn clear(&self, dataset_id: Option<&str>) -> Result<usize, PortError> {
        let _guard = self.lock.write().map_err(|_| {
            PortError::CacheError("Falha de concorrência ao limpar HiveParquetStore".into())
        })?;

        let mut removed = 0;
        if let Some(id) = dataset_id {
            let target_dir = self.base_dir.join(id);
            if target_dir.exists() {
                fs::remove_dir_all(&target_dir).map_err(PortError::IoError)?;
                removed += 1;
            }
        } else if self.base_dir.exists() {
            let entries = fs::read_dir(&self.base_dir).map_err(PortError::IoError)?;
            for entry in entries {
                let entry = entry.map_err(PortError::IoError)?;
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(&path).map_err(PortError::IoError)?;
                    removed += 1;
                } else if path.is_file() {
                    fs::remove_file(&path).map_err(PortError::IoError)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    /// Remove snapshots com data de criação estritamente anterior a `cutoff`.
    /// Exclui os arquivos Parquet associados do disco e atualiza os catálogos `snapshots.json`.
    /// Retorna a contagem de snapshots removidos.
    pub fn clear_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize, PortError> {
        let _guard = self.lock.write().map_err(|_| {
            PortError::CacheError("Falha de concorrência ao expirar HiveParquetStore".into())
        })?;

        if !self.base_dir.exists() {
            return Ok(0);
        }

        let mut total_purged = 0;
        let entries = fs::read_dir(&self.base_dir).map_err(PortError::IoError)?;

        for entry in entries {
            let entry = entry.map_err(PortError::IoError)?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dataset_id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) if !name.starts_with('.') => name.to_string(),
                _ => continue,
            };

            let catalog_path = self.catalog_path(&dataset_id);
            if !catalog_path.exists() {
                continue;
            }

            let mut catalog = self.load_catalog(&dataset_id)?;
            let original_count = catalog.snapshots.len();

            let mut retained = Vec::with_capacity(original_count);
            for snap in catalog.snapshots {
                if snap.created_at < cutoff {
                    // Remover arquivo Parquet físico
                    let full_parquet = self.base_dir.join(&snap.file_path);
                    if full_parquet.exists() {
                        let _ = fs::remove_file(&full_parquet);
                        // Tentar remover diretório snapshot vazio pai
                        if let Some(parent) = full_parquet.parent() {
                            let _ = fs::remove_dir(parent);
                        }
                    }
                    total_purged += 1;
                } else {
                    retained.push(snap);
                }
            }

            if retained.len() != original_count {
                catalog.snapshots = retained;
                let serialized = serde_json::to_string_pretty(&catalog)
                    .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
                fs::write(&catalog_path, serialized).map_err(PortError::IoError)?;
            }
        }

        Ok(total_purged)
    }

    /// Calcula estatísticas do cache: total de bytes em disco e contagem total de snapshots.
    pub fn status(&self, dataset_id: Option<&str>) -> Result<CacheStorageStatus, PortError> {
        let target_dir = match dataset_id {
            Some(id) => self.base_dir.join(id),
            None => self.base_dir.clone(),
        };

        if !target_dir.exists() {
            return Ok(CacheStorageStatus {
                total_bytes: 0,
                snapshot_count: 0,
                base_path: self.base_dir.to_string_lossy().to_string(),
            });
        }

        let mut total_bytes = 0u64;
        let mut snapshot_count = 0usize;

        fn dir_size(path: &Path, bytes: &mut u64) -> std::io::Result<()> {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let p = entry.path();
                    if p.is_dir() {
                        dir_size(&p, bytes)?;
                    } else if p.is_file() {
                        *bytes += entry.metadata()?.len();
                    }
                }
            }
            Ok(())
        }

        let _ = dir_size(&target_dir, &mut total_bytes);

        if let Some(id) = dataset_id {
            let cat = self.load_catalog(id)?;
            snapshot_count = cat.snapshots.len();
        } else if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir()
                    && let Some(name) = p.file_name().and_then(|n| n.to_str())
                    && !name.starts_with('.')
                    && let Ok(cat) = self.load_catalog(name)
                {
                    snapshot_count += cat.snapshots.len();
                }
            }
        }

        Ok(CacheStorageStatus {
            total_bytes,
            snapshot_count,
            base_path: self.base_dir.to_string_lossy().to_string(),
        })
    }

    fn catalog_path(&self, dataset_id: &str) -> PathBuf {
        self.base_dir
            .join(dataset_id)
            .join("_metadata")
            .join("snapshots.json")
    }

    fn load_catalog(&self, dataset_id: &str) -> Result<SnapshotCatalog, PortError> {
        let path = self.catalog_path(dataset_id);
        if !path.exists() {
            return Ok(SnapshotCatalog::default());
        }

        let content = fs::read_to_string(&path).map_err(PortError::IoError)?;
        serde_json::from_str(&content).map_err(|e| {
            PortError::TabularDecodeError(format!("Erro ao desserializar catálogo: {e}"))
        })
    }

    fn append_to_catalog(
        &self,
        dataset_id: &str,
        record: &SnapshotRecord,
    ) -> Result<(), PortError> {
        let mut catalog = self.load_catalog(dataset_id)?;
        catalog.snapshots.push(record.clone());

        let path = self.catalog_path(dataset_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(PortError::IoError)?;
        }

        let serialized = serde_json::to_string_pretty(&catalog)
            .map_err(|e| PortError::TabularDecodeError(e.to_string()))?;
        fs::write(&path, serialized).map_err(PortError::IoError)?;

        Ok(())
    }
}

/// Sumário das estatísticas do cache Hive-Parquet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheStorageStatus {
    /// Tamanho total ocupado no disco em bytes.
    pub total_bytes: u64,
    /// Total de snapshots registrados.
    pub snapshot_count: usize,
    /// Caminho base do diretório de cache.
    pub base_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{StringArray, UInt32Array};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn create_sample_batch() -> RecordBatch {
        let names = Arc::new(StringArray::from(vec!["Acre", "Rio Branco"]));
        let codes = Arc::new(UInt32Array::from(vec![12, 1200401]));
        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("ibge_code", DataType::UInt32, false),
        ]));
        RecordBatch::try_new(schema, vec![names, codes]).unwrap()
    }

    #[test]
    fn test_hive_parquet_save_and_time_travel() {
        let temp = tempdir().unwrap();
        let store = HiveParquetStore::new(temp.path()).unwrap();

        let batch1 = create_sample_batch();
        let record1 = store
            .save_batch("datasus.sim", "AC", 2022, &batch1)
            .unwrap();
        assert_eq!(record1.num_rows, 2);
        assert_eq!(record1.sha256_parquet.len(), 64);

        // Ler snapshot específico por UUID
        let retrieved = store
            .read_snapshot("datasus.sim", record1.snapshot_id)
            .unwrap();
        assert_eq!(retrieved.num_rows(), 2);
        assert_eq!(retrieved.num_columns(), 2);

        // Time travel as of futuro -> encontra record1
        let future = Utc::now() + chrono::Duration::hours(1);
        let found = store.read_as_of("datasus.sim", "AC", 2022, future).unwrap();
        assert!(found.is_some());

        // Time travel as of passado distante -> não encontra
        let past = Utc::now() - chrono::Duration::hours(1);
        let not_found = store.read_as_of("datasus.sim", "AC", 2022, past).unwrap();
        assert!(not_found.is_none());

        // Listar snapshots
        let all = store.list_snapshots("datasus.sim").unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_hive_parquet_clear_and_expiration() {
        let temp = tempdir().unwrap();
        let store = HiveParquetStore::new(temp.path()).unwrap();

        let batch = create_sample_batch();
        store.save_batch("datasus.sih", "SP", 2023, &batch).unwrap();
        store.save_batch("datasus.sim", "RJ", 2023, &batch).unwrap();

        let status = store.status(None).unwrap();
        assert_eq!(status.snapshot_count, 2);
        assert!(status.total_bytes > 0);

        // Limpeza de uma única fonte
        let removed_sih = store.clear(Some("datasus.sih")).unwrap();
        assert_eq!(removed_sih, 1);

        let status_after_sih = store.status(None).unwrap();
        assert_eq!(status_after_sih.snapshot_count, 1);

        // Limpeza total
        let removed_all = store.clear(None).unwrap();
        assert!(removed_all >= 1);

        let status_empty = store.status(None).unwrap();
        assert_eq!(status_empty.snapshot_count, 0);
    }
}
