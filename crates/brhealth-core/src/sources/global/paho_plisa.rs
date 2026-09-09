// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do PLISA (PAHO / OPAS - Plataforma de Informação em Saúde das Américas).

use std::sync::Arc;

use arrow::array::{ArrayRef, StringBuilder, UInt16Builder, UInt32Builder, UInt8Builder};
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
use crate::sources::datasus::helpers::{get_str_value, get_u16_value, get_u32_value, get_u8_value};

/// Adaptador SPI para a plataforma PLISA da OPAS (vigilância transfronteiriça de arboviroses).
#[derive(Debug, Default, Clone)]
pub struct PahoPlisaDataSource;

impl PahoPlisaDataSource {
    /// Cria uma nova instância de `PahoPlisaDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de notificações epidemiológicas para o schema canônico da OPAS/PLISA.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_panamerican_surveillance_schema();

        // 1. report_id (report_id ou id)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "report_id", i)
                .or_else(|| get_str_value(raw_batch, "id", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("PLISA_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. disease_name (disease ou agravo)
        let mut disease_builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
        for i in 0..num_rows {
            let d = get_str_value(raw_batch, "disease", i)
                .or_else(|| get_str_value(raw_batch, "agravo", i))
                .unwrap_or("Dengue");
            disease_builder.append_value(d);
        }
        let disease_col: ArrayRef = Arc::new(disease_builder.finish());

        // 3. country_iso3 (country_code ou iso3)
        let mut country_builder = StringBuilder::with_capacity(num_rows, num_rows * 3);
        for i in 0..num_rows {
            let c = get_str_value(raw_batch, "country_code", i)
                .or_else(|| get_str_value(raw_batch, "iso3", i))
                .unwrap_or("BRA");
            country_builder.append_value(c);
        }
        let country_col: ArrayRef = Arc::new(country_builder.finish());

        // 4. subnational_iso (subnational_code ou state)
        let mut sub_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            if let Some(s) = get_str_value(raw_batch, "subnational_code", i)
                .or_else(|| get_str_value(raw_batch, "state", i))
            {
                sub_builder.append_value(s);
            } else {
                sub_builder.append_null();
            }
        }
        let sub_col: ArrayRef = Arc::new(sub_builder.finish());

        // 5. epidemiological_year (epi_year ou ano)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "epi_year", i)
                .or_else(|| get_u16_value(raw_batch, "ano", i))
                .unwrap_or(2024);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 6. epidemiological_week (epi_week ou se)
        let mut wk_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let wk = get_u8_value(raw_batch, "epi_week", i)
                .or_else(|| get_u8_value(raw_batch, "se", i))
                .unwrap_or(1);
            wk_builder.append_value(wk);
        }
        let wk_col: ArrayRef = Arc::new(wk_builder.finish());

        // 7. suspected_cases (suspected ou casos_suspeitos)
        let mut susp_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let s = get_u32_value(raw_batch, "suspected", i)
                .or_else(|| get_u32_value(raw_batch, "casos_suspeitos", i))
                .unwrap_or(0);
            susp_builder.append_value(s);
        }
        let susp_col: ArrayRef = Arc::new(susp_builder.finish());

        // 8. confirmed_cases (confirmed ou casos_confirmados)
        let mut conf_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let c = get_u32_value(raw_batch, "confirmed", i)
                .or_else(|| get_u32_value(raw_batch, "casos_confirmados", i))
                .unwrap_or(0);
            conf_builder.append_value(c);
        }
        let conf_col: ArrayRef = Arc::new(conf_builder.finish());

        // 9. severe_cases (severe ou casos_graves)
        let mut sev_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let s = get_u32_value(raw_batch, "severe", i)
                .or_else(|| get_u32_value(raw_batch, "casos_graves", i))
                .unwrap_or(0);
            sev_builder.append_value(s);
        }
        let sev_col: ArrayRef = Arc::new(sev_builder.finish());

        // 10. deaths (deaths ou obitos)
        let mut death_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let d = get_u32_value(raw_batch, "deaths", i)
                .or_else(|| get_u32_value(raw_batch, "obitos", i))
                .unwrap_or(0);
            death_builder.append_value(d);
        }
        let death_col: ArrayRef = Arc::new(death_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col,
                disease_col,
                country_col,
                sub_col,
                yr_col,
                wk_col,
                susp_col,
                conf_col,
                sev_col,
                death_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for PahoPlisaDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "global.paho_plisa",
            display_name: "PAHO / OPAS PLISA - Plataforma de Informação de Saúde das Américas",
            maintaining_agency: "Organização Pan-Americana da Saúde (OPAS / PAHO)",
            scope: GeographicScope::Supranational {
                entity: "PAHO_AMRO".into(),
            },
            category: SourceCategory::ClinicalMorbidity,
            temporal_resolution: "Semanal",
            spatial_resolution: "País / Província / Estado (Américas)",
            supported_years: 2014..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_panamerican_surveillance_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let country = params.jurisdiction_code.as_deref().unwrap_or("BRA");
        let year = params.year;

        Ok(format!(
            "https://opendata.paho.org/plisa/arbovirus/{year}/{country}.parquet"
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
