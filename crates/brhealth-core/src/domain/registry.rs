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
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn register<S: HealthDataSourceSPI>(&mut self, source: S) {
        let meta = source.metadata();
        self.sources.insert(meta.id.to_string(), Arc::new(source));
    }

    pub fn register_pack(&mut self, pack: Vec<Arc<dyn HealthDataSourceSPI>>) {
        for source in pack {
            let meta = source.metadata();
            self.sources.insert(meta.id.to_string(), source);
        }
    }

    pub fn get(&self, source_id: &str) -> Result<Arc<dyn HealthDataSourceSPI>, PortError> {
        self.sources
            .get(source_id)
            .cloned()
            .ok_or_else(|| PortError::ResourceNotFound(format!("Fonte '{}' não registrada", source_id)))
    }

    pub fn list_by_scope(&self, scope: &GeographicScope) -> Vec<SourceMetadata> {
        self.sources
            .values()
            .map(|s| s.metadata())
            .filter(|m| &m.scope == scope)
            .collect()
    }
}
