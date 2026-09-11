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

    /// Verifica se uma fonte específica está registrada pelo seu ID (suporta alias com `.` ou `_`).
    #[must_use]
    pub fn contains(&self, source_id: &str) -> bool {
        if self.sources.contains_key(source_id) {
            return true;
        }
        let dot_normalized = source_id.replace('_', ".");
        if self.sources.contains_key(&dot_normalized) {
            return true;
        }
        let underscore_normalized = source_id.replace('.', "_");
        self.sources.contains_key(&underscore_normalized)
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

    /// Obtém uma fonte registrada pelo ID, aceitando formatos com ponto (`datasus.sih`) ou underscore (`datasus_sih`).
    pub fn get(&self, source_id: &str) -> Result<Arc<dyn HealthDataSourceSPI>, PortError> {
        if let Some(source) = self.sources.get(source_id) {
            return Ok(source.clone());
        }

        let dot_normalized = source_id.replace('_', ".");
        if let Some(source) = self.sources.get(&dot_normalized) {
            return Ok(source.clone());
        }

        let underscore_normalized = source_id.replace('.', "_");
        if let Some(source) = self.sources.get(&underscore_normalized) {
            return Ok(source.clone());
        }

        let mut available_ids: Vec<_> = self.sources.keys().map(String::as_str).collect();
        available_ids.sort_unstable();

        let suggestion = available_ids
            .iter()
            .map(|&id| (id, levenshtein_distance(source_id, id)))
            .min_by_key(|&(_, dist)| dist)
            .filter(|&(_, dist)| dist <= 5)
            .map(|(best_match, _)| format!(" Você quis dizer '{best_match}'?"))
            .unwrap_or_default();

        let available_str = available_ids.join(", ");
        Err(PortError::ResourceNotFound(format!(
            "Fonte '{source_id}' não registrada.{suggestion} Fontes disponíveis: [{available_str}]"
        )))
    }

    /// Lista as fontes filtrando pelo escopo geográfico.
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

#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut distances = vec![vec![0usize; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        distances[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        distances[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            distances[i][j] = (distances[i - 1][j] + 1)
                .min(distances[i][j - 1] + 1)
                .min(distances[i - 1][j - 1] + cost);
        }
    }
    distances[a_chars.len()][b_chars.len()]
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
