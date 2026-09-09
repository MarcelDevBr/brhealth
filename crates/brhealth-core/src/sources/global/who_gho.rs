// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Global Health Observatory (WHO / OMS).

use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Builder, StringBuilder, UInt16Builder};
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
use crate::sources::datasus::helpers::{get_float64_value, get_str_value, get_u16_value};

/// Adaptador SPI para os indicadores mundiais de saúde da OMS (WHO GHO).
#[derive(Debug, Default, Clone)]
pub struct WhoGhoDataSource;

impl WhoGhoDataSource {
    /// Cria uma nova instância de `WhoGhoDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de indicadores da OMS para o schema canônico de indicadores globais.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_who_indicator_schema();

        // 1. indicator_code (IndicatorCode)
        let mut code_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let code = get_str_value(raw_batch, "IndicatorCode", i).unwrap_or("WHOSIS_000001");
            code_builder.append_value(code);
        }
        let code_col: ArrayRef = Arc::new(code_builder.finish());

        // 2. indicator_name (IndicatorName)
        let mut name_builder = StringBuilder::with_capacity(num_rows, num_rows * 30);
        for i in 0..num_rows {
            let name = get_str_value(raw_batch, "IndicatorName", i)
                .unwrap_or("Life expectancy at birth (years)");
            name_builder.append_value(name);
        }
        let name_col: ArrayRef = Arc::new(name_builder.finish());

        // 3. country_iso3 (SpatialDim / Country)
        let mut country_builder = StringBuilder::with_capacity(num_rows, num_rows * 3);
        for i in 0..num_rows {
            let c = get_str_value(raw_batch, "SpatialDim", i)
                .or_else(|| get_str_value(raw_batch, "Country", i))
                .unwrap_or("GLOBAL");
            country_builder.append_value(c);
        }
        let country_col: ArrayRef = Arc::new(country_builder.finish());

        // 4. reference_year (TimeDim / Year)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "TimeDim", i)
                .or_else(|| get_u16_value(raw_batch, "Year", i))
                .unwrap_or(2023);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 5. sex (Dim1 / Sex)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            if let Some(s) = get_str_value(raw_batch, "Dim1", i)
                .or_else(|| get_str_value(raw_batch, "Sex", i))
            {
                sex_builder.append_value(s);
            } else {
                sex_builder.append_null();
            }
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 6. numeric_value (NumericValue / Value)
        let mut val_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let val = get_float64_value(raw_batch, "NumericValue", i)
                .or_else(|| get_float64_value(raw_batch, "Value", i))
                .unwrap_or(0.0);
            val_builder.append_value(val);
        }
        let val_col: ArrayRef = Arc::new(val_builder.finish());

        // 7. low_bound_value (Low)
        let mut low_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(l) = get_float64_value(raw_batch, "Low", i) {
                low_builder.append_value(l);
            } else {
                low_builder.append_null();
            }
        }
        let low_col: ArrayRef = Arc::new(low_builder.finish());

        // 8. high_bound_value (High)
        let mut high_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(h) = get_float64_value(raw_batch, "High", i) {
                high_builder.append_value(h);
            } else {
                high_builder.append_null();
            }
        }
        let high_col: ArrayRef = Arc::new(high_builder.finish());

        // 9. sdg_target_id (SDGTarget)
        let mut sdg_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            if let Some(sdg) = get_str_value(raw_batch, "SDGTarget", i) {
                sdg_builder.append_value(sdg);
            } else {
                sdg_builder.append_null();
            }
        }
        let sdg_col: ArrayRef = Arc::new(sdg_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                code_col,
                name_col,
                country_col,
                yr_col,
                sex_col,
                val_col,
                low_col,
                high_col,
                sdg_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for WhoGhoDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.who_gho",
            display_name: "WHO GHO - Global Health Observatory",
            maintaining_agency: "World Health Organization (OMS / WHO)",
            scope: GeographicScope::Supranational {
                entity: "WHO_GLOBAL".into(),
            },
            category: SourceCategory::GlobalBurdenIndicators,
            temporal_resolution: "Anual",
            spatial_resolution: "País (ISO 3166-1 alpha-3)",
            supported_years: 1950..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_who_indicator_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let indicator = params
            .extra_filters
            .get("indicator")
            .map(|s| s.as_str())
            .unwrap_or("WHOSIS_000001");
        let year = params.year;

        Ok(format!(
            "https://ghoapi.azureedge.net/api/{indicator}?$filter=SpatialDimType eq 'COUNTRY' and TimeDim eq {year}"
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
