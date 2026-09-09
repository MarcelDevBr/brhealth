// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Global Burden of Disease (IHME / University of Washington).

use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Builder, StringBuilder, UInt8Builder, UInt16Builder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::sources::datasus::helpers::{
    get_float64_value, get_str_value, get_u8_value, get_u16_value,
};

/// Adaptador SPI para métricas de carga global de doenças (GBD / IHME).
#[derive(Debug, Default, Clone)]
pub struct IhmeGbdDataSource;

impl IhmeGbdDataSource {
    /// Cria uma nova instância de `IhmeGbdDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de dados de carga global para o schema canônico do GBD.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_global_burden_schema();

        // 1. measure_name (measure_name / MEASURE)
        let mut measure_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            let m = get_str_value(raw_batch, "measure_name", i)
                .or_else(|| get_str_value(raw_batch, "MEASURE", i))
                .unwrap_or("DALYs");
            measure_builder.append_value(m);
        }
        let measure_col: ArrayRef = Arc::new(measure_builder.finish());

        // 2. cause_code (cause_id / CAUSE_CODE)
        let mut cause_code_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            let c = get_str_value(raw_batch, "cause_id", i)
                .or_else(|| get_str_value(raw_batch, "CAUSE_CODE", i))
                .unwrap_or("B.1");
            cause_code_builder.append_value(c);
        }
        let cause_code_col: ArrayRef = Arc::new(cause_code_builder.finish());

        // 3. cause_name (cause_name / CAUSE)
        let mut cause_name_builder = StringBuilder::with_capacity(num_rows, num_rows * 20);
        for i in 0..num_rows {
            let n = get_str_value(raw_batch, "cause_name", i)
                .or_else(|| get_str_value(raw_batch, "CAUSE", i))
                .unwrap_or("Cardiovascular diseases");
            cause_name_builder.append_value(n);
        }
        let cause_name_col: ArrayRef = Arc::new(cause_name_builder.finish());

        // 4. country_iso3 (location_id / COUNTRY)
        let mut country_builder = StringBuilder::with_capacity(num_rows, num_rows * 3);
        for i in 0..num_rows {
            let c = get_str_value(raw_batch, "location_id", i)
                .or_else(|| get_str_value(raw_batch, "COUNTRY", i))
                .unwrap_or("BRA");
            country_builder.append_value(c);
        }
        let country_col: ArrayRef = Arc::new(country_builder.finish());

        // 5. subnational_code (subnational_id / STATE)
        let mut sub_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            if let Some(s) = get_str_value(raw_batch, "subnational_id", i)
                .or_else(|| get_str_value(raw_batch, "STATE", i))
            {
                sub_builder.append_value(s);
            } else {
                sub_builder.append_null();
            }
        }
        let sub_col: ArrayRef = Arc::new(sub_builder.finish());

        // 6. year (year / ANO)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "year", i)
                .or_else(|| get_u16_value(raw_batch, "ANO", i))
                .unwrap_or(2021);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 7. age_group_id (age_id / AGE_GROUP)
        let mut age_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let a = get_u8_value(raw_batch, "age_id", i)
                .or_else(|| get_u8_value(raw_batch, "AGE_GROUP", i))
                .unwrap_or(22); // Ex: All ages
            age_builder.append_value(a);
        }
        let age_col: ArrayRef = Arc::new(age_builder.finish());

        // 8. sex (sex_id / SEXO)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows);
        for i in 0..num_rows {
            let s = get_str_value(raw_batch, "sex_id", i)
                .or_else(|| get_str_value(raw_batch, "SEXO", i))
                .unwrap_or("Both");
            sex_builder.append_value(s);
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 9. metric_value (val / VALOR)
        let mut val_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let val = get_float64_value(raw_batch, "val", i)
                .or_else(|| get_float64_value(raw_batch, "VALOR", i))
                .unwrap_or(0.0);
            val_builder.append_value(val);
        }
        let val_col: ArrayRef = Arc::new(val_builder.finish());

        // 10. metric_rate_per_100k (rate / TAXA)
        let mut rate_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let rate = get_float64_value(raw_batch, "rate", i)
                .or_else(|| get_float64_value(raw_batch, "TAXA", i))
                .unwrap_or(0.0);
            rate_builder.append_value(rate);
        }
        let rate_col: ArrayRef = Arc::new(rate_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                measure_col,
                cause_code_col,
                cause_name_col,
                country_col,
                sub_col,
                yr_col,
                age_col,
                sex_col,
                val_col,
                rate_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for IhmeGbdDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.ihme_gbd",
            display_name: "IHME GBD - Global Burden of Disease Study",
            maintaining_agency: "Institute for Health Metrics and Evaluation (IHME / Univ. Washington)",
            scope: GeographicScope::Supranational {
                entity: "IHME_GLOBAL".into(),
            },
            category: SourceCategory::GlobalBurdenIndicators,
            temporal_resolution: "Anual",
            spatial_resolution: "País / Estado Subnacional",
            supported_years: 1990..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_global_burden_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        let country = params.jurisdiction_code.as_deref().unwrap_or("BRA");

        Ok(format!(
            "https://ghdx.healthdata.org/record/ihme-data/gbd-{year}-{country}.parquet"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        let decoder = DbfDecoder::new();
        let raw_batch = match decoder.decode_to_record_batch(&raw_bytes) {
            Ok(b) => b,
            Err(_) => RecordBatch::new_empty(self.target_schema()),
        };

        let harmonized = if raw_batch.num_rows() > 0 {
            self.harmonize_batch(&raw_batch)?
        } else {
            raw_batch
        };

        Ok(vec![harmonized])
    }
}
