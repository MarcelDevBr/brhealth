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
pub mod ffi;
pub mod infrastructure;
pub mod sources;

pub use domain::application::{
    BRHealthApplicationService, PipelineExecutionOptions, PipelineExecutionResult,
};
pub use domain::ports::outbound::PortError;
pub use domain::provenance::FairManifest;
pub use domain::registry::SourceRegistry;
pub use domain::schema::CanonicalSchemas;
pub use domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
pub use domain::spatial::{
    append_h3_column, coord_to_h3_index, h3_grid_disk, h3_grid_distance, h3_index_to_coord,
};
pub use domain::transforms::ibge::{
    calculate_ibge_dv, harmonize_ibge_code, reconcile_historical_ibge_code,
};
pub use domain::transforms::pipeline::{BatchTransformationStep, TransformationPipeline};
pub use infrastructure::cache::{default_cache_dir, default_data_dir};
pub use infrastructure::transport::{
    DataFreshness, FetchOutcome, HealthReport, ResiliencePolicy, ResilientTransport,
    SourceHealthChecker, SourceHealthEntry, SourceStatus,
};
pub use sources::{create_pack_brasil, create_pack_global};
