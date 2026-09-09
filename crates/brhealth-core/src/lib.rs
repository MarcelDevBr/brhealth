// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! # BRHealth: Motor Analítico de Alta Performance para Saúde Coletiva e Determinantes Sociais
//!
//! O BRHealth implementa a arquitetura Hexagonal Orientada a Dados (Hexagonal DOD),
//! garantindo memória colunar contígua via Apache Arrow, interoperabilidade Zero-Copy,
//! descompressão nativa segura e proveniência científica estrita (W3C PROV-O).

pub mod decoders;
pub mod domain;
pub mod infrastructure;
pub mod sources;

pub use domain::ports::outbound::PortError;
pub use domain::provenance::FairManifest;
pub use domain::registry::SourceRegistry;
pub use domain::schema::CanonicalSchemas;
pub use domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
pub use domain::transforms::ibge::{calculate_ibge_dv, harmonize_ibge_code};
