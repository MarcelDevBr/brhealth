// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub engine: EngineMetadata,
    pub execution_metadata: ExecutionMetadata,
    pub sources: Vec<SourceProvenance>,
    pub pipeline_steps: Vec<PipelineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetadata {
    pub name: String,
    pub version: String,
    pub rustc_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub run_uuid: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub reproducibility_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceProvenance {
    pub source_name: String,
    pub scope: String,
    pub uri: String,
    pub sha256_raw_payload: String,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub step: usize,
    pub operator: String,
    pub details: serde_json::Value,
}

impl FairManifest {
    pub fn new(sources: Vec<SourceProvenance>, steps: Vec<PipelineStep>) -> Self {
        Self {
            schema: "https://brhealth.org/schemas/v1/fair-manifest.json".into(),
            engine: EngineMetadata {
                name: "brhealth-core".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                rustc_version: "rustc 1.97.1".into(),
            },
            execution_metadata: ExecutionMetadata {
                run_uuid: Uuid::new_v4(),
                timestamp_utc: Utc::now(),
                reproducibility_tier: "StrictDeterministic".into(),
            },
            sources,
            pipeline_steps: steps,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
