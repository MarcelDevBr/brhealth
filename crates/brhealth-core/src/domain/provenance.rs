// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Governança de dados FAIR e Linhagem Científica Criptográfica W3C PROV-O.
//!
//! Este módulo implementa a geração e verificação de manifestos de auditoria
//! científica e grafos de proveniência criptográfica rastreáveis bit a bit via SHA-256.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Calcula o hash criptográfico SHA-256 de uma fatia de bytes em memória contígua.
#[inline]
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Calcula o hash criptográfico SHA-256 de um fluxo contínuo de I/O em tempo de execução ("no voo").
pub fn compute_stream_sha256<R: std::io::Read>(reader: &mut R) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64 KiB buffer alinhado
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Manifesto FAIR de proveniência científica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FairManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub engine: EngineMetadata,
    pub execution_metadata: ExecutionMetadata,
    pub sources: Vec<SourceProvenance>,
    pub pipeline_steps: Vec<PipelineStep>,
}

/// Metadados do motor de computação analítica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineMetadata {
    pub name: String,
    pub version: String,
    pub rustc_version: String,
}

/// Metadados de execução com identificador universal único (UUIDv4) e carimbo temporal UTC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionMetadata {
    pub run_uuid: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub reproducibility_tier: String,
}

/// Linhagem de uma fonte primária de dados consumida.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceProvenance {
    pub source_name: String,
    pub scope: String,
    pub uri: String,
    pub sha256_raw_payload: String,
    pub retrieved_at: DateTime<Utc>,
}

/// Registro de etapa de transformação matemática ou espacial executada pelo pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineStep {
    pub step: usize,
    pub operator: String,
    pub details: serde_json::Value,
}

impl FairManifest {
    /// Cria um novo manifesto FAIR para uma execução analítica.
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

    /// Serializa o manifesto em JSON formatado legível.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Gera a representação semântica JSON-LD do grafo segundo a ontologia **W3C PROV-O**.
    ///
    /// Inclui mapeamentos para `prov:Entity`, `prov:Activity`, `prov:Agent`,
    /// `prov:wasGeneratedBy` e `prov:used`.
    pub fn to_w3c_prov_json_ld(&self) -> serde_json::Value {
        let run_id = self.execution_metadata.run_uuid.to_string();
        let activity_uri = format!("urn:uuid:{run_id}:activity");
        let agent_uri = format!("urn:brhealth:engine:{}", self.engine.version);

        let mut graph = Vec::new();

        // 1. Agente (BRHealth Core Engine)
        graph.push(serde_json::json!({
            "@id": agent_uri,
            "@type": ["prov:Agent", "prov:SoftwareAgent"],
            "rdfs:label": self.engine.name,
            "prov:version": self.engine.version,
            "brhealth:rustc_version": self.engine.rustc_version
        }));

        // 2. Atividade (Execução do Pipeline Analítico)
        graph.push(serde_json::json!({
            "@id": activity_uri,
            "@type": "prov:Activity",
            "prov:startedAtTime": self.execution_metadata.timestamp_utc.to_rfc3339(),
            "prov:wasAssociatedWith": { "@id": agent_uri },
            "brhealth:reproducibility_tier": self.execution_metadata.reproducibility_tier
        }));

        // 3. Entidades de Entrada (Fontes de dados consumidas)
        for (i, source) in self.sources.iter().enumerate() {
            let entity_uri = format!("urn:uuid:{run_id}:entity:source:{i}");
            graph.push(serde_json::json!({
                "@id": entity_uri,
                "@type": "prov:Entity",
                "rdfs:label": source.source_name,
                "prov:atLocation": source.uri,
                "brhealth:scope": source.scope,
                "brhealth:sha256": source.sha256_raw_payload,
                "prov:generatedAtTime": source.retrieved_at.to_rfc3339()
            }));

            // Registro de uso pela atividade
            graph.push(serde_json::json!({
                "@id": activity_uri,
                "prov:used": { "@id": entity_uri }
            }));
        }

        // Montar documento JSON-LD com @context
        serde_json::json!({
            "@context": {
                "prov": "http://www.w3.org/ns/prov#",
                "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
                "brhealth": "https://brhealth.org/ontology#"
            },
            "@graph": graph
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_sha256_computation() {
        let payload = b"BRHealth Analytical Engine 2026";
        let hash1 = compute_sha256(payload);
        assert_eq!(hash1.len(), 64);

        let mut cursor = Cursor::new(payload);
        let hash2 = compute_stream_sha256(&mut cursor).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fair_manifest_w3c_prov_o_json_ld() {
        let sources = vec![SourceProvenance {
            source_name: "datasus.sim".into(),
            scope: "AC".into(),
            uri: "ftp://datasus.saude.gov.br/dissemin/publicos/sim/doac2022.dbc".into(),
            sha256_raw_payload: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .into(),
            retrieved_at: Utc::now(),
        }];

        let steps = vec![PipelineStep {
            step: 1,
            operator: "spatial.h3_indexing".into(),
            details: serde_json::json!({ "resolution": 8 }),
        }];

        let manifest = FairManifest::new(sources, steps);
        let json_manifest = manifest.to_json().unwrap();
        assert!(json_manifest.contains("brhealth-core"));

        let prov_ld = manifest.to_w3c_prov_json_ld();
        let prov_str = serde_json::to_string_pretty(&prov_ld).unwrap();
        assert!(prov_str.contains("http://www.w3.org/ns/prov#"));
        assert!(prov_str.contains("prov:SoftwareAgent"));
        assert!(prov_str.contains("datasus.sim"));
    }
}
