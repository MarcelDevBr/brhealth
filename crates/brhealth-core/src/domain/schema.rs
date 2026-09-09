// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use std::sync::Arc;

pub struct CanonicalSchemas;

impl CanonicalSchemas {
    /// Schema Canônico para Estatísticas Vitais de Mortalidade (SIM, CDC Wonder, WHO)
    pub fn canonical_mortality_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("event_date", DataType::Date32, false),
            Field::new("underlying_cause_icd10", DataType::Utf8, false),
            Field::new("underlying_cause_icd11", DataType::Utf8, true),
            Field::new("age_years", DataType::UInt16, true),
            Field::new("sex", DataType::Utf8, true),
            Field::new("race_ethnicity", DataType::Utf8, true),
            Field::new("maternal_death", DataType::Boolean, true),
        ]))
    }

    /// Schema Canônico para Morbidade Hospitalar (SIHSUS RD/AIH)
    pub fn canonical_hospital_morbidity_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("municipality_residence", DataType::Utf8, false),
            Field::new("municipality_hospital", DataType::Utf8, false),
            Field::new("admission_date", DataType::Date32, false),
            Field::new("discharge_date", DataType::Date32, false),
            Field::new("length_of_stay_days", DataType::UInt16, false),
            Field::new("main_diagnosis_icd10", DataType::Utf8, false),
            Field::new("secondary_diagnosis_icd10", DataType::Utf8, true),
            Field::new("procedure_sigtap", DataType::Utf8, false),
            Field::new("total_paid_amount", DataType::Float64, false),
            Field::new("icu_days", DataType::UInt16, false),
            Field::new("death_outcome", DataType::Boolean, false),
            Field::new("is_csap", DataType::Boolean, false),
        ]))
    }

    /// Schema Canônico para Reanálise Climática e Meteorologia (INMET / ERA5)
    pub fn canonical_climate_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("station_or_grid_id", DataType::Utf8, false),
            Field::new(
                "timestamp_utc",
                DataType::Timestamp(TimeUnit::Second, Some("UTC".into())),
                false,
            ),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
            Field::new("h3_index_res7", DataType::UInt64, false),
            Field::new("temperature_mean_c", DataType::Float32, true),
            Field::new("temperature_max_c", DataType::Float32, true),
            Field::new("temperature_min_c", DataType::Float32, true),
            Field::new("relative_humidity_percent", DataType::Float32, true),
            Field::new("precipitation_total_mm", DataType::Float32, true),
            Field::new("solar_radiation_kj_m2", DataType::Float32, true),
        ]))
    }
}
