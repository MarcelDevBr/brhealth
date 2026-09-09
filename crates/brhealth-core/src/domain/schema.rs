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

    /// Schema Canônico para Estatísticas Vitais de Nascidos Vivos (SINASC)
    pub fn canonical_birth_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("birth_date", DataType::Date32, false),
            Field::new("birth_weight_grams", DataType::UInt16, true),
            Field::new("gestational_weeks", DataType::UInt8, true),
            Field::new("apgar_1min", DataType::UInt8, true),
            Field::new("apgar_5min", DataType::UInt8, true),
            Field::new("sex", DataType::Utf8, true),
            Field::new("race_ethnicity", DataType::Utf8, true),
            Field::new("mother_age_years", DataType::UInt8, true),
            Field::new("delivery_type", DataType::Utf8, true),
            Field::new("congenital_anomaly", DataType::Boolean, true),
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

    /// Schema Canônico para Doenças e Agravos de Notificação (SINAN)
    pub fn canonical_notifiable_disease_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("notification_id", DataType::Utf8, false),
            Field::new("disease_code", DataType::Utf8, false),
            Field::new("notification_date", DataType::Date32, false),
            Field::new("symptom_onset_date", DataType::Date32, true),
            Field::new("patient_municipality", DataType::Utf8, false),
            Field::new("notification_municipality", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("age_years", DataType::UInt16, true),
            Field::new("sex", DataType::Utf8, true),
            Field::new("diagnostic_criterion", DataType::Utf8, true),
            Field::new("case_classification", DataType::Utf8, true),
            Field::new("closure_outcome", DataType::Utf8, true),
        ]))
    }

    /// Schema Canônico para Produção Ambulatorial do SUS (SIASUS BPA/APAC)
    pub fn canonical_ambulatory_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("patient_municipality", DataType::Utf8, false),
            Field::new("facility_cnes", DataType::Utf8, false),
            Field::new("facility_municipality", DataType::Utf8, false),
            Field::new("procedure_sigtap", DataType::Utf8, false),
            Field::new("service_date", DataType::Date32, false),
            Field::new("main_diagnosis_icd10", DataType::Utf8, true),
            Field::new("quantity_produced", DataType::UInt32, false),
            Field::new("total_paid_amount", DataType::Float64, false),
            Field::new("patient_sex", DataType::Utf8, true),
            Field::new("patient_age_years", DataType::UInt16, true),
        ]))
    }

    /// Schema Canônico para Estabelecimentos de Saúde e Infraestrutura (CNES)
    pub fn canonical_health_facility_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("cnes_id", DataType::Utf8, false),
            Field::new("facility_name", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("management_type", DataType::Utf8, false),
            Field::new("facility_type_code", DataType::Utf8, false),
            Field::new("has_emergency_care", DataType::Boolean, false),
            Field::new("total_surgical_beds", DataType::UInt16, false),
            Field::new("total_clinical_beds", DataType::UInt16, false),
            Field::new("total_icu_beds_sus", DataType::UInt16, false),
            Field::new("total_icu_beds_non_sus", DataType::UInt16, false),
            Field::new("competence_year_month", DataType::Utf8, false),
        ]))
    }

    /// Schema Canônico para Vigilância Imunológica e Vacinas (SI-PNI / RNDS)
    pub fn canonical_immunization_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("vaccination_event_id", DataType::Utf8, false),
            Field::new("vaccine_code", DataType::Utf8, false),
            Field::new("vaccine_name", DataType::Utf8, false),
            Field::new("dose_order", DataType::Utf8, false),
            Field::new("vaccination_date", DataType::Date32, false),
            Field::new("patient_municipality", DataType::Utf8, false),
            Field::new("vaccination_facility_cnes", DataType::Utf8, false),
            Field::new("lot_number", DataType::Utf8, true),
            Field::new("patient_age_years", DataType::UInt8, true),
            Field::new("patient_sex", DataType::Utf8, true),
        ]))
    }

    /// Schema Canônico para Vigilância Alimentar e Nutricional (SISVAN)
    pub fn canonical_nutritional_surveillance_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("assessment_id", DataType::Utf8, false),
            Field::new("assessment_date", DataType::Date32, false),
            Field::new("patient_municipality", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("age_months", DataType::UInt16, false),
            Field::new("sex", DataType::Utf8, false),
            Field::new("weight_kg", DataType::Float32, false),
            Field::new("height_cm", DataType::Float32, false),
            Field::new("bmi", DataType::Float32, true),
            Field::new("who_growth_classification", DataType::Utf8, true),
        ]))
    }

    /// Schema Canônico para Rastreamento e Diagnóstico de Câncer (SISCAN / SISCOLO / SISMAMA)
    pub fn canonical_cancer_screening_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("exam_id", DataType::Utf8, false),
            Field::new("cancer_type", DataType::Utf8, false), // "CERVICAL" ou "BREAST"
            Field::new("exam_date", DataType::Date32, false),
            Field::new("patient_municipality", DataType::Utf8, false),
            Field::new("patient_age_years", DataType::UInt8, false),
            Field::new("clinical_indication", DataType::Utf8, true),
            Field::new("diagnostic_result", DataType::Utf8, false),
            Field::new("biopsy_recommended", DataType::Boolean, false),
        ]))
    }

    /// Schema Canônico para Banco de Preços em Saúde e Fármacos (BPS / CMED)
    pub fn canonical_drug_price_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("purchase_id", DataType::Utf8, false),
            Field::new("procurement_date", DataType::Date32, false),
            Field::new("buyer_jurisdiction", DataType::Utf8, false),
            Field::new("active_ingredient", DataType::Utf8, false),
            Field::new("atc_code", DataType::Utf8, true),
            Field::new("dosage_form", DataType::Utf8, false),
            Field::new("quantity_purchased", DataType::UInt32, false),
            Field::new("unit_price_brl", DataType::Float64, false),
            Field::new("total_price_brl", DataType::Float64, false),
        ]))
    }

    /// Schema Canônico para Demografia Censitária (IBGE Censo)
    pub fn canonical_demographic_census_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("census_sector_id", DataType::Utf8, false),
            Field::new("municipality_code", DataType::Utf8, false),
            Field::new("census_year", DataType::UInt16, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("total_population", DataType::UInt32, false),
            Field::new("male_population", DataType::UInt32, false),
            Field::new("female_population", DataType::UInt32, false),
            Field::new("total_private_households", DataType::UInt32, false),
            Field::new("median_household_income_brl", DataType::Float32, true),
        ]))
    }

    /// Schema Canônico para Condições Socioeconômicas Amostrais (IBGE PNAD Contínua)
    pub fn canonical_socioeconomic_pnad_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("survey_id", DataType::Utf8, false),
            Field::new("survey_year", DataType::UInt16, false),
            Field::new("survey_quarter", DataType::UInt8, false),
            Field::new("state_code", DataType::Utf8, false),
            Field::new("sample_weight", DataType::Float64, false),
            Field::new("head_of_household_sex", DataType::Utf8, false),
            Field::new("per_capita_household_income", DataType::Float64, true),
            Field::new("has_private_health_insurance", DataType::Boolean, false),
            Field::new("education_level_years", DataType::UInt8, true),
        ]))
    }

    /// Schema Canônico para Vulnerabilidade Social (CadÚnico / MDS)
    pub fn canonical_social_vulnerability_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("family_id", DataType::Utf8, false),
            Field::new("municipality_code", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("is_extreme_poverty", DataType::Boolean, false),
            Field::new("receives_income_transfer", DataType::Boolean, false),
            Field::new("number_of_family_members", DataType::UInt8, false),
            Field::new("has_piped_water", DataType::Boolean, false),
            Field::new("has_sewage_network", DataType::Boolean, false),
            Field::new("has_electricity", DataType::Boolean, false),
        ]))
    }

    /// Schema Canônico para Focos de Calor e Fumaça (BDQueimadas / INPE)
    pub fn canonical_wildfire_smoke_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("fire_event_id", DataType::Utf8, false),
            Field::new("satellite_sensor", DataType::Utf8, false),
            Field::new(
                "detection_timestamp_utc",
                DataType::Timestamp(TimeUnit::Second, Some("UTC".into())),
                false,
            ),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
            Field::new("h3_index_res8", DataType::UInt64, false),
            Field::new("municipality_code", DataType::Utf8, false),
            Field::new("biome_name", DataType::Utf8, false),
            Field::new("fire_radiative_power_mw", DataType::Float32, true),
            Field::new("estimated_pm25_ug_m3", DataType::Float32, true),
        ]))
    }

    /// Schema Canônico para Vigilância da Qualidade da Água (SISAGUA / SNIS)
    pub fn canonical_water_quality_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("sample_id", DataType::Utf8, false),
            Field::new("collection_date", DataType::Date32, false),
            Field::new("municipality_code", DataType::Utf8, false),
            Field::new("water_supply_system_id", DataType::Utf8, false),
            Field::new("sampling_point_type", DataType::Utf8, false),
            Field::new("total_coliforms_detected", DataType::Boolean, false),
            Field::new("escherichia_coli_detected", DataType::Boolean, false),
            Field::new("free_residual_chlorine_mg_l", DataType::Float32, true),
            Field::new("turbidity_ntu", DataType::Float32, true),
            Field::new("fluoride_mg_l", DataType::Float32, true),
        ]))
    }

    /// Schema Canônico para Indicadores Globais da Saúde (WHO GHO)
    pub fn canonical_who_indicator_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("indicator_code", DataType::Utf8, false),
            Field::new("indicator_name", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("reference_year", DataType::UInt16, false),
            Field::new("sex", DataType::Utf8, true),
            Field::new("numeric_value", DataType::Float64, false),
            Field::new("low_bound_value", DataType::Float64, true),
            Field::new("high_bound_value", DataType::Float64, true),
            Field::new("sdg_target_id", DataType::Utf8, true),
        ]))
    }

    /// Schema Canônico para Carga Global de Doenças (IHME GBD)
    pub fn canonical_global_burden_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("measure_name", DataType::Utf8, false), // "DALYs", "YLDs", "YLLs", "Deaths"
            Field::new("cause_code", DataType::Utf8, false),
            Field::new("cause_name", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("subnational_code", DataType::Utf8, true),
            Field::new("year", DataType::UInt16, false),
            Field::new("age_group_id", DataType::UInt8, false),
            Field::new("sex", DataType::Utf8, false),
            Field::new("metric_value", DataType::Float64, false),
            Field::new("metric_rate_per_100k", DataType::Float64, false),
        ]))
    }

    /// Schema Canônico para Demografia em Grade Contínua (WorldPop)
    pub fn canonical_gridded_population_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("grid_cell_id", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, false),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
            Field::new("year", DataType::UInt16, false),
            Field::new("estimated_population_count", DataType::Float64, false),
            Field::new("population_density_sq_km", DataType::Float64, false),
        ]))
    }

    /// Schema Canônico para Vigilância Pan-Americana de Arboviroses (PAHO / PLISA)
    pub fn canonical_panamerican_surveillance_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("report_id", DataType::Utf8, false),
            Field::new("disease_name", DataType::Utf8, false), // "Dengue", "Chikungunya", "Zika", "Oropouche"
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("subnational_iso", DataType::Utf8, true),
            Field::new("epidemiological_year", DataType::UInt16, false),
            Field::new("epidemiological_week", DataType::UInt8, false),
            Field::new("suspected_cases", DataType::UInt32, false),
            Field::new("confirmed_cases", DataType::UInt32, false),
            Field::new("severe_cases", DataType::UInt32, false),
            Field::new("deaths", DataType::UInt32, false),
        ]))
    }

    /// Schema Canônico para Pesquisa de Orçamentos Familiares (IBGE POF)
    pub fn canonical_pof_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("household_id", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("reference_year", DataType::UInt16, false),
            Field::new("total_monthly_income", DataType::Float64, false),
            Field::new("health_expenditure_total", DataType::Float64, false),
            Field::new("medication_expenditure", DataType::Float64, false),
            Field::new("health_insurance_expenditure", DataType::Float64, false),
            Field::new("catastrophic_expenditure_flag", DataType::Boolean, false),
        ]))
    }

    /// Schema Canônico para Pesquisa Nacional de Saúde do Escolar (IBGE PeNSE)
    pub fn canonical_pense_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("student_id", DataType::Utf8, false),
            Field::new("school_id", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("survey_year", DataType::UInt16, false),
            Field::new("age_years", DataType::UInt8, false),
            Field::new("sex", DataType::Utf8, false),
            Field::new("tobacco_use_past_30d", DataType::Boolean, false),
            Field::new("alcohol_use_past_30d", DataType::Boolean, false),
            Field::new("physical_activity_minutes_weekly", DataType::UInt16, false),
            Field::new("soda_consumption_daily", DataType::Boolean, false),
        ]))
    }

    /// Schema Canônico para Perfil dos Municípios Brasileiros (IBGE MUNIC)
    pub fn canonical_munic_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("survey_year", DataType::UInt16, false),
            Field::new("has_municipal_health_plan", DataType::Boolean, false),
            Field::new("has_municipal_health_fund", DataType::Boolean, false),
            Field::new("has_health_council", DataType::Boolean, false),
            Field::new("emergency_contingency_plan", DataType::Boolean, false),
            Field::new("primary_care_teams_count", DataType::UInt32, false),
        ]))
    }

    /// Schema Canônico para Monitoramento de Cobertura Vegetal e Desmatamento (INPE PRODES)
    pub fn canonical_prodes_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("polygon_id", DataType::Utf8, false),
            Field::new("jurisdiction_code", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("biome_name", DataType::Utf8, false),
            Field::new("year", DataType::UInt16, false),
            Field::new("deforested_area_sq_km", DataType::Float64, false),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
        ]))
    }

    /// Schema Canônico para Monitoramento Global da Qualidade do Ar (OpenAQ)
    pub fn canonical_openaq_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("measurement_id", DataType::Utf8, false),
            Field::new("location_name", DataType::Utf8, false),
            Field::new("country_iso3", DataType::Utf8, false),
            Field::new("h3_index_res8", DataType::UInt64, true),
            Field::new("timestamp_utc", DataType::Timestamp(TimeUnit::Second, Some("UTC".into())), false),
            Field::new("pollutant", DataType::Utf8, false), // "pm25", "pm10", "no2", "o3", "so2", "co"
            Field::new("value_micrograms_m3", DataType::Float32, false),
            Field::new("latitude", DataType::Float64, false),
            Field::new("longitude", DataType::Float64, false),
        ]))
    }
}
