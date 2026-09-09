// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::ports::outbound::{
    DecompressorPort, LocalCachePort, PortError, SyncStatePort, TransportPort,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GeographicScope {
    National { iso_3166_alpha3: String },
    Subnational { iso_3166_2: String },
    Supranational { entity: String },
    GlobalGrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceCategory {
    ClinicalMorbidity,
    VitalStatistics,
    SocioDemographic,
    EnvironmentalPlanetary,
    AssistanceInfrastructure,
    FinancialAdministrative,
    GlobalBurdenIndicators,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata {
    pub id: &'static str,
    pub display_name: &'static str,
    pub maintaining_agency: &'static str,
    pub scope: GeographicScope,
    pub category: SourceCategory,
    pub temporal_resolution: &'static str,
    pub spatial_resolution: &'static str,
    pub supported_years: RangeInclusive<u16>,
    pub requires_authentication: bool,
}

#[derive(Debug, Clone)]
pub struct DataQueryParams {
    pub scope: GeographicScope,
    pub jurisdiction_code: Option<String>,
    pub year: u16,
    pub month: Option<u8>,
    pub extra_filters: HashMap<String, String>,
    pub as_of_snapshot: Option<DateTime<Utc>>,
}

pub struct SourceExecutionContext {
    pub transport: Arc<dyn TransportPort>,
    pub decompressor: Arc<dyn DecompressorPort>,
    pub cache: Arc<dyn LocalCachePort>,
    pub state: Arc<dyn SyncStatePort>,
}

#[async_trait]
pub trait HealthDataSourceSPI: Send + Sync + 'static {
    fn metadata(&self) -> SourceMetadata;
    fn target_schema(&self) -> Arc<Schema>;
    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError>;
    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError>;
}
