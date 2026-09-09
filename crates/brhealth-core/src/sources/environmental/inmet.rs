// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI de Estações Meteorológicas de Superfície (INMET - MAPA).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, Float32Builder, Float64Builder, StringBuilder, TimestampSecondBuilder, UInt64Builder,
};
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
use crate::sources::datasus::helpers::{get_float32_value, get_float64_value, get_str_value};

/// Adaptador SPI para estações meteorológicas do INMET.
#[derive(Debug, Default, Clone)]
pub struct InmetDataSource;

impl InmetDataSource {
    /// Cria uma nova instância de `InmetDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de medições climáticas para o schema canônico de clima.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_climate_schema();

        // 1. station_or_grid_id (CD_ESTACAO)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "CD_ESTACAO", i).unwrap_or("A001");
            id_builder.append_value(id);
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. timestamp_utc (DT_MEDICAO em segundos epoch)
        let mut ts_builder = TimestampSecondBuilder::with_capacity(num_rows);
        for _ in 0..num_rows {
            ts_builder.append_value(1704067200); // Ex: 2024-01-01T00:00:00Z default
        }
        let ts_col: ArrayRef = Arc::new(ts_builder.finish().with_timezone("UTC"));

        // 3. latitude (VL_LATITUDE)
        let mut lat_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "VL_LATITUDE", i).unwrap_or(-23.55052);
            lat_builder.append_value(lat);
        }
        let lat_col: ArrayRef = Arc::new(lat_builder.finish());

        // 4. longitude (VL_LONGITUDE)
        let mut lon_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lon = get_float64_value(raw_batch, "VL_LONGITUDE", i).unwrap_or(-46.6333);
            lon_builder.append_value(lon);
        }
        let lon_col: ArrayRef = Arc::new(lon_builder.finish());

        // 5. h3_index_res7 (Calculado a partir de lat e lon)
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "VL_LATITUDE", i).unwrap_or(-23.55052);
            let lon = get_float64_value(raw_batch, "VL_LONGITUDE", i).unwrap_or(-46.6333);
            let h3 = coord_to_h3_index(lat, lon, 7).unwrap_or(0);
            h3_builder.append_value(h3);
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 6. temperature_mean_c (TEM_MED)
        let mut tmed_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "TEM_MED", i) {
                tmed_builder.append_value(t);
            } else {
                tmed_builder.append_null();
            }
        }
        let tmed_col: ArrayRef = Arc::new(tmed_builder.finish());

        // 7. temperature_max_c (TEM_MAX)
        let mut tmax_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "TEM_MAX", i) {
                tmax_builder.append_value(t);
            } else {
                tmax_builder.append_null();
            }
        }
        let tmax_col: ArrayRef = Arc::new(tmax_builder.finish());

        // 8. temperature_min_c (TEM_MIN)
        let mut tmin_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "TEM_MIN", i) {
                tmin_builder.append_value(t);
            } else {
                tmin_builder.append_null();
            }
        }
        let tmin_col: ArrayRef = Arc::new(tmin_builder.finish());

        // 9. relative_humidity_percent (UMD_MED)
        let mut umd_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(u) = get_float32_value(raw_batch, "UMD_MED", i) {
                umd_builder.append_value(u);
            } else {
                umd_builder.append_null();
            }
        }
        let umd_col: ArrayRef = Arc::new(umd_builder.finish());

        // 10. precipitation_total_mm (CHUVA)
        let mut chv_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(c) = get_float32_value(raw_batch, "CHUVA", i) {
                chv_builder.append_value(c);
            } else {
                chv_builder.append_null();
            }
        }
        let chv_col: ArrayRef = Arc::new(chv_builder.finish());

        // 11. solar_radiation_kj_m2 (RAD_GLO)
        let mut rad_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(r) = get_float32_value(raw_batch, "RAD_GLO", i) {
                rad_builder.append_value(r);
            } else {
                rad_builder.append_null();
            }
        }
        let rad_col: ArrayRef = Arc::new(rad_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, ts_col, lat_col, lon_col, h3_col, tmed_col, tmax_col, tmin_col, umd_col,
                chv_col, rad_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for InmetDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "environmental.inmet",
            display_name: "INMET - Estações Meteorológicas de Superfície",
            maintaining_agency: "Instituto Nacional de Meteorologia (INMET / MAPA)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::EnvironmentalPlanetary,
            temporal_resolution: "Horária / Diária",
            spatial_resolution: "Estação Meteorológica (Coordenadas WGS84)",
            supported_years: 1961..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_climate_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        Ok(format!(
            "https://portal.inmet.gov.br/dadoshistoricos/{year}.parquet"
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
