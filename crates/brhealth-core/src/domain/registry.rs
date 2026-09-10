// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use std::collections::HashMap;
use std::sync::Arc;

use super::ports::outbound::PortError;
use super::source_spi::{GeographicScope, HealthDataSourceSPI, SourceMetadata};

#[derive(Default)]
pub struct SourceRegistry {
    sources: HashMap<String, Arc<dyn HealthDataSourceSPI>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria uma instância pré-populada com todos os pacotes oficiais (Brasil e Global).
    #[must_use]
    pub fn standard() -> Self {
        let mut reg = Self::default();
        reg.extend(crate::sources::create_pack_brasil());
        reg.extend(crate::sources::create_pack_global());
        reg
    }

    /// Quantidade de fontes registradas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Verifica se não há fontes registradas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Verifica se uma fonte específica está registrada pelo seu ID.
    #[must_use]
    pub fn contains(&self, source_id: &str) -> bool {
        self.sources.contains_key(source_id)
    }

    pub fn register<S: HealthDataSourceSPI>(&mut self, source: S) {
        let meta = source.metadata();
        self.sources.insert(meta.id.to_string(), Arc::new(source));
    }

    pub fn register_pack<I: IntoIterator<Item = Arc<dyn HealthDataSourceSPI>>>(&mut self, pack: I) {
        for source in pack {
            let meta = source.metadata();
            self.sources.insert(meta.id.to_string(), source);
        }
    }

    pub fn get(&self, source_id: &str) -> Result<Arc<dyn HealthDataSourceSPI>, PortError> {
        self.sources.get(source_id).cloned().ok_or_else(|| {
            PortError::ResourceNotFound(format!("Fonte '{source_id}' não registrada"))
        })
    }

    pub fn list_by_scope(&self, scope: &GeographicScope) -> Vec<SourceMetadata> {
        self.sources
            .values()
            .map(|s| s.metadata())
            .filter(|m| &m.scope == scope)
            .collect()
    }

    /// Lista os metadados de todas as fontes registradas.
    #[must_use]
    pub fn list_all(&self) -> Vec<SourceMetadata> {
        self.sources.values().map(|s| s.metadata()).collect()
    }
}

impl Extend<Arc<dyn HealthDataSourceSPI>> for SourceRegistry {
    fn extend<T: IntoIterator<Item = Arc<dyn HealthDataSourceSPI>>>(&mut self, iter: T) {
        self.register_pack(iter);
    }
}

impl FromIterator<Arc<dyn HealthDataSourceSPI>> for SourceRegistry {
    fn from_iter<T: IntoIterator<Item = Arc<dyn HealthDataSourceSPI>>>(iter: T) -> Self {
        let mut reg = Self::default();
        reg.extend(iter);
        reg
    }
}
