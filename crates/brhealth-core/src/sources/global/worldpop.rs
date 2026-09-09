// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do WorldPop (University of Southampton - Demografia em Grade Contínua).

use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Builder, StringBuilder, UInt16Builder, UInt64Builder};
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
use crate::domain::spatial::coord_to_h3_index;
use crate::sources::datasus::helpers::{
    get_float64_value, get_str_value, get_u16_value,
};

/// Adaptador SPI para população contínua em grade (WorldPop).
#[derive(Debug, Default, Clone)]
pub struct WorldPopDataSource;

impl WorldPopDataSource {
    /// Cria uma nova instância de `WorldPopDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de células para o schema canônico de população em grade.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_gridded_population_schema();

        // 1. grid_cell_id (cell_id ou pixel_id)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "cell_id", i)
                .or_else(|| get_str_value(raw_batch, "pixel_id", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("WPOP_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. country_iso3 (iso3 / country)
        let mut country_builder = StringBuilder::with_capacity(num_rows, num_rows * 3);
        for i in 0..num_rows {
            let c = get_str_value(raw_batch, "iso3", i)
                .or_else(|| get_str_value(raw_batch, "country", i))
                .unwrap_or("BRA");
            country_builder.append_value(c);
        }
        let country_col: ArrayRef = Arc::new(country_builder.finish());

        // 3. h3_index_res8
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "latitude", i).unwrap_or(0.0);
            let lon = get_float64_value(raw_batch, "longitude", i).unwrap_or(0.0);
            let h3 = coord_to_h3_index(lat, lon, 8).unwrap_or(0);
            h3_builder.append_value(h3);
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 4. latitude (latitude / y)
        let mut lat_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "latitude", i)
                .or_else(|| get_float64_value(raw_batch, "y", i))
                .unwrap_or(0.0);
            lat_builder.append_value(lat);
        }
        let lat_col: ArrayRef = Arc::new(lat_builder.finish());

        // 5. longitude (longitude / x)
        let mut lon_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lon = get_float64_value(raw_batch, "longitude", i)
                .or_else(|| get_float64_value(raw_batch, "x", i))
                .unwrap_or(0.0);
            lon_builder.append_value(lon);
        }
        let lon_col: ArrayRef = Arc::new(lon_builder.finish());

        // 6. year (year / ano)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "year", i)
                .or_else(|| get_u16_value(raw_batch, "ano", i))
                .unwrap_or(2020);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 7. estimated_population_count (population / pop)
        let mut pop_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let p = get_float64_value(raw_batch, "population", i)
                .or_else(|| get_float64_value(raw_batch, "pop", i))
                .unwrap_or(0.0);
            pop_builder.append_value(p);
        }
        let pop_col: ArrayRef = Arc::new(pop_builder.finish());

        // 8. population_density_sq_km (density / pop_density)
        let mut dens_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let d = get_float64_value(raw_batch, "density", i)
                .or_else(|| get_float64_value(raw_batch, "pop_density", i))
                .unwrap_or(0.0);
            dens_builder.append_value(d);
        }
        let dens_col: ArrayRef = Arc::new(dens_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col,
                country_col,
                h3_col,
                lat_col,
                lon_col,
                yr_col,
                pop_col,
                dens_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for WorldPopDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.worldpop",
            display_name: "WorldPop - High Resolution Population Mapping",
            maintaining_agency: "WorldPop Project / University of Southampton",
            scope: GeographicScope::GlobalGrid,
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Anual",
            spatial_resolution: "Grade Contínua 100m / H3 Resolução 8",
            supported_years: 2000..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_gridded_population_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let country = params.jurisdiction_code.as_deref().unwrap_or("BRA");
        let year = params.year;

        Ok(format!(
            "https://data.worldpop.org/GIS/Population/Global_2000_2020/{year}/{country}/{country}_ppp_{year}.parquet"
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
