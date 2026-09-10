// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações Hospitalares (SIHSUS - DATASUS RD/AIH).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanArray, BooleanBuilder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_date32_col, build_f64_col, build_harmonized_ibge_col, build_record_id_col,
    build_str_col, build_str_opt_col, build_u16_col, get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o SIHSUS (AIH Reduzida - RD) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SihDataSource;

use crate::domain::schema::default_values::PREFIX_AIH;

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
        let record_id_col = build_record_id_col(raw_batch, "N_AIH", PREFIX_AIH, num_rows);

        // 2. municipality_residence (MUNIC_RES)
        let municipality_residence_col =
            build_harmonized_ibge_col(raw_batch, "MUNIC_RES", num_rows);

        // 3. municipality_hospital (MUNIC_MOV)
        let municipality_hospital_col =
            build_harmonized_ibge_col(raw_batch, "MUNIC_MOV", num_rows);

        // 4. admission_date (DT_INTER)
        let admission_date_col = build_date32_col(raw_batch, "DT_INTER", 0, num_rows);

        // 5. discharge_date (DT_SAIDA)
        let discharge_date_col = build_date32_col(raw_batch, "DT_SAIDA", 0, num_rows);

        // 6. length_of_stay_days (DIAS_PERM)
        let stay_col = build_u16_col(raw_batch, "DIAS_PERM", 0, num_rows);

        // 7. main_diagnosis_icd10 (DIAG_PRINC)
        let diag_p_col = build_str_col(raw_batch, "DIAG_PRINC", "Z00", num_rows);

        // 8. secondary_diagnosis_icd10 (DIAG_SECUN)
        let diag_s_col = build_str_opt_col(raw_batch, "DIAG_SECUN", 5, num_rows);

        // 9. procedure_sigtap (PROC_REA)
        let proc_col = build_str_col(raw_batch, "PROC_REA", "0000000000", num_rows);

        // 10. total_paid_amount (VAL_TOT)
        let val_col = build_f64_col(raw_batch, "VAL_TOT", 0.0, num_rows);

        // 11. icu_days (UTI_MES_TO)
        let uti_col = build_u16_col(raw_batch, "UTI_MES_TO", 0, num_rows);

        // 12. death_outcome (MORTE: 1=Sim, 0=Não)
        let mut morte_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let died = get_str_value(raw_batch, "MORTE", i) == Some("1");
            morte_builder.append_value(died);
        }
        let morte_col: ArrayRef = Arc::new(morte_builder.finish());

        // 13. is_csap (Inicialmente falso até enriquecimento com módulo CSAP)
        let csap_col: ArrayRef = Arc::new(BooleanArray::from(vec![false; num_rows]));

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

    fn mirror_uris(&self, params: &DataQueryParams) -> Vec<String> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year_2digits = params.year % 100;
        let month = params.month.unwrap_or(1);
        vec![
            format!(
                "https://datasus.saude.gov.br/transferencia-download-de-arquivos/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
            ),
            format!(
                "ftp://ftp2.datasus.gov.br/dissemin/publicos/SIHSUS/200801_/Dados/RD{uf}{year_2digits:02}{month:02}.dbc"
            ),
        ]
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
