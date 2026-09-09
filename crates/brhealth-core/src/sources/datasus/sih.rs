// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações Hospitalares (SIHSUS - DATASUS RD/AIH).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, Float64Builder, StringBuilder, UInt16Builder,
};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_date32_value, get_float64_value, get_str_value};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SIHSUS (AIH Reduzida - RD) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SihDataSource;

impl SihDataSource {
    /// Cria uma nova instância de `SihDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de morbidade hospitalar.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_hospital_morbidity_schema();

        // 1. record_id (N_AIH)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 14);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "N_AIH", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("AIH_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let record_id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. municipality_residence (MUNIC_RES)
        let mut res_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "MUNIC_RES", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            res_builder.append_value(resolved);
        }
        let municipality_residence_col: ArrayRef = Arc::new(res_builder.finish());

        // 3. municipality_hospital (MUNIC_MOV)
        let mut mov_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "MUNIC_MOV", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mov_builder.append_value(resolved);
        }
        let municipality_hospital_col: ArrayRef = Arc::new(mov_builder.finish());

        // 4. admission_date (DT_INTER)
        let mut adm_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let date_val = get_date32_value(raw_batch, "DT_INTER", i).unwrap_or(0);
            adm_builder.append_value(date_val);
        }
        let admission_date_col: ArrayRef = Arc::new(adm_builder.finish());

        // 5. discharge_date (DT_SAIDA)
        let mut dis_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let date_val = get_date32_value(raw_batch, "DT_SAIDA", i).unwrap_or(0);
            dis_builder.append_value(date_val);
        }
        let discharge_date_col: ArrayRef = Arc::new(dis_builder.finish());

        // 6. length_of_stay_days (DIAS_PERM)
        let mut stay_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let stay_val = get_str_value(raw_batch, "DIAS_PERM", i)
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
            stay_builder.append_value(stay_val);
        }
        let stay_col: ArrayRef = Arc::new(stay_builder.finish());

        // 7. main_diagnosis_icd10 (DIAG_PRINC)
        let mut diag_p_builder = StringBuilder::with_capacity(num_rows, num_rows * 5);
        for i in 0..num_rows {
            let diag = get_str_value(raw_batch, "DIAG_PRINC", i).unwrap_or("Z00");
            diag_p_builder.append_value(diag);
        }
        let diag_p_col: ArrayRef = Arc::new(diag_p_builder.finish());

        // 8. secondary_diagnosis_icd10 (DIAG_SECUN)
        let mut diag_s_builder = StringBuilder::with_capacity(num_rows, num_rows * 5);
        for i in 0..num_rows {
            let diag = get_str_value(raw_batch, "DIAG_SECUN", i);
            if let Some(d) = diag {
                diag_s_builder.append_value(d);
            } else {
                diag_s_builder.append_null();
            }
        }
        let diag_s_col: ArrayRef = Arc::new(diag_s_builder.finish());

        // 9. procedure_sigtap (PROC_REA)
        let mut proc_builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
        for i in 0..num_rows {
            let proc_str = get_str_value(raw_batch, "PROC_REA", i).unwrap_or("0000000000");
            proc_builder.append_value(proc_str);
        }
        let proc_col: ArrayRef = Arc::new(proc_builder.finish());

        // 10. total_paid_amount (VAL_TOT)
        let mut val_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let amount = get_float64_value(raw_batch, "VAL_TOT", i).unwrap_or(0.0);
            val_builder.append_value(amount);
        }
        let val_col: ArrayRef = Arc::new(val_builder.finish());

        // 11. icu_days (UTI_MES_TO)
        let mut uti_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let uti_val = get_str_value(raw_batch, "UTI_MES_TO", i)
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
            uti_builder.append_value(uti_val);
        }
        let uti_col: ArrayRef = Arc::new(uti_builder.finish());

        // 12. death_outcome (MORTE: 1=Sim, 0=Não)
        let mut morte_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let died = get_str_value(raw_batch, "MORTE", i) == Some("1");
            morte_builder.append_value(died);
        }
        let morte_col: ArrayRef = Arc::new(morte_builder.finish());

        // 13. is_csap (Inicialmente falso até enriquecimento com módulo CSAP)
        let mut csap_builder = BooleanBuilder::with_capacity(num_rows);
        for _ in 0..num_rows {
            csap_builder.append_value(false);
        }
        let csap_col: ArrayRef = Arc::new(csap_builder.finish());

        let columns = vec![
            record_id_col,
            municipality_residence_col,
            municipality_hospital_col,
            admission_date_col,
            discharge_date_col,
            stay_col,
            diag_p_col,
            diag_s_col,
            proc_col,
            val_col,
            uti_col,
            morte_col,
            csap_col,
        ];

        RecordBatch::try_new(target_schema, columns).map_err(|e| {
            PortError::TabularDecodeError(format!(
                "Falha ao gerar RecordBatch canônico de morbidade hospitalar: {e}"
            ))
        })
    }
}

#[async_trait]
impl HealthDataSourceSPI for SihDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sih",
            display_name: "Sistema de Informações Hospitalares (SIHSUS RD/AIH - DATASUS)",
            maintaining_agency: "Ministério da Saúde (Brasil)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            category: SourceCategory::ClinicalMorbidity,
            temporal_resolution: "Mensal / Competência",
            spatial_resolution: "Municipal (IBGE 7 dígitos)",
            supported_years: 1998..=2024,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_hospital_morbidity_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year_2digits = params.year % 100;
        let month = params.month.unwrap_or(1);
        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        let decompressed_dbf = context.decompressor.decompress(&raw_bytes)?;

        let dbf_decoder = DbfDecoder::new();
        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed_dbf)?;

        let canonical_batch = self.harmonize_batch(&raw_batch)?;

        Ok(vec![canonical_batch])
    }
}
