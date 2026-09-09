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
}
