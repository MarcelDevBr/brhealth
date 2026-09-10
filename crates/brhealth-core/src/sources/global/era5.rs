// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Copernicus ERA5 (ECMWF / União Europeia - Reanálise Climática Global).

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

/// Adaptador SPI para reanálise climática global do Copernicus ERA5.
#[derive(Debug, Default, Clone)]
pub struct Era5DataSource;

impl Era5DataSource {
    /// Cria uma nova instância de `Era5DataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de células climáticas para o schema canônico de clima.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_climate_schema();

        // 1. station_or_grid_id (cell_id / grid_id)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "cell_id", i)
                .or_else(|| get_str_value(raw_batch, "grid_id", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("ERA5_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. timestamp_utc
        let mut ts_builder = TimestampSecondBuilder::with_capacity(num_rows);
        for _ in 0..num_rows {
            ts_builder.append_value(1704067200);
        }
        let ts_col: ArrayRef = Arc::new(ts_builder.finish().with_timezone("UTC"));

        // 3. latitude (latitude / lat)
        let mut lat_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "latitude", i)
                .or_else(|| get_float64_value(raw_batch, "lat", i))
                .unwrap_or(0.0);
            lat_builder.append_value(lat);
        }
        let lat_col: ArrayRef = Arc::new(lat_builder.finish());

        // 4. longitude (longitude / lon)
        let mut lon_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lon = get_float64_value(raw_batch, "longitude", i)
                .or_else(|| get_float64_value(raw_batch, "lon", i))
                .unwrap_or(0.0);
            lon_builder.append_value(lon);
        }
        let lon_col: ArrayRef = Arc::new(lon_builder.finish());

        // 5. h3_index_res7
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let lat = get_float64_value(raw_batch, "latitude", i).unwrap_or(0.0);
            let lon = get_float64_value(raw_batch, "longitude", i).unwrap_or(0.0);
            let h3 = coord_to_h3_index(lat, lon, 7).unwrap_or(0);
            h3_builder.append_value(h3);
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 6. temperature_mean_c (t2m_c ou temp_mean)
        let mut tmed_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let t = get_float32_value(raw_batch, "t2m_c", i)
                .or_else(|| get_float32_value(raw_batch, "temp_mean", i))
                .unwrap_or(24.0);
            tmed_builder.append_value(t);
        }
        let tmed_col: ArrayRef = Arc::new(tmed_builder.finish());

        // 7. temperature_max_c (t2m_max)
        let mut tmax_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "t2m_max", i) {
                tmax_builder.append_value(t);
            } else {
                tmax_builder.append_null();
            }
        }
        let tmax_col: ArrayRef = Arc::new(tmax_builder.finish());

        // 8. temperature_min_c (t2m_min)
        let mut tmin_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "t2m_min", i) {
                tmin_builder.append_value(t);
            } else {
                tmin_builder.append_null();
            }
        }
        let tmin_col: ArrayRef = Arc::new(tmin_builder.finish());

        // 9. relative_humidity_percent (rh)
        let mut rh_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(r) = get_float32_value(raw_batch, "rh", i) {
                rh_builder.append_value(r);
            } else {
                rh_builder.append_null();
            }
        }
        let rh_col: ArrayRef = Arc::new(rh_builder.finish());

        // 10. precipitation_total_mm (tp_mm)
        let mut prec_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(p) = get_float32_value(raw_batch, "tp_mm", i) {
                prec_builder.append_value(p);
            } else {
                prec_builder.append_null();
            }
        }
        let prec_col: ArrayRef = Arc::new(prec_builder.finish());

        // 11. solar_radiation_kj_m2 (ssrd)
        let mut rad_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(s) = get_float32_value(raw_batch, "ssrd", i) {
                rad_builder.append_value(s);
            } else {
                rad_builder.append_null();
            }
        }
        let rad_col: ArrayRef = Arc::new(rad_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, ts_col, lat_col, lon_col, h3_col, tmed_col, tmax_col, tmin_col, rh_col,
                prec_col, rad_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for Era5DataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.copernicus_era5",
            display_name: "Copernicus ERA5-Land Reanalysis",
            maintaining_agency: "European Centre for Medium-Range Weather Forecasts (ECMWF)",
            scope: GeographicScope::GlobalGrid,
            category: SourceCategory::EnvironmentalPlanetary,
            temporal_resolution: "Horária / Diária / Mensal",
            spatial_resolution: "Grade Geodésica 0.1° / H3 Resolução 7",
            supported_years: 1950..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_climate_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        Ok(format!(
            "https://cds.climate.copernicus.eu/api/v2/resources/reanalysis-era5-land-{year}.parquet"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        if raw_bytes.is_empty() {
            return Ok(vec![RecordBatch::new_empty(self.target_schema())]);
        }

        // 1. Verifica se o payload é um arquivo Apache Parquet (Magic number 'PAR1')
        if raw_bytes.len() >= 4 && &raw_bytes[0..4] == b"PAR1" {
            use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

            let mem_bytes = bytes::Bytes::from(raw_bytes);
            let builder = ParquetRecordBatchReaderBuilder::try_new(mem_bytes).map_err(|e| {
                PortError::TabularDecodeError(format!("Falha ao ler cabeçalho Parquet ERA5: {e}"))
            })?;
            let reader = builder.build().map_err(|e| {
                PortError::TabularDecodeError(format!(
                    "Falha ao construir leitor Parquet ERA5: {e}"
                ))
            })?;

            let mut harmonized_batches = Vec::new();
            for batch_res in reader {
                let batch = batch_res.map_err(|e| {
                    PortError::TabularDecodeError(format!(
                        "Erro na leitura de batch Parquet ERA5: {e}"
                    ))
                })?;
                let harmonized = self.harmonize_batch(&batch)?;
                harmonized_batches.push(harmonized);
            }
            return Ok(harmonized_batches);
        }

        // 2. Fallback para decodificador DBF
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
