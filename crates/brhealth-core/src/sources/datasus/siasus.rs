// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações Ambulatoriais (SIASUS - DATASUS BPA/APAC).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, Date32Builder, Float64Builder, StringBuilder, UInt16Builder, UInt32Builder,
};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    get_date32_value, get_float64_value, get_str_value, get_u16_value, get_u32_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SIASUS (Produção Ambulatorial) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SiasusDataSource;

impl SiasusDataSource {
    /// Cria uma nova instância de `SiasusDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico ambulatorial.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_ambulatory_schema();

        // 1. record_id (PA_DOC_ID ou gerado)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "PA_DOC_ID", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("AMB_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let record_id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. patient_municipality (PA_MUNPCN)
        let mut pcn_mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "PA_MUNPCN", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            pcn_mun_builder.append_value(resolved);
        }
        let patient_mun_col: ArrayRef = Arc::new(pcn_mun_builder.finish());

        // 3. facility_cnes (PA_CODUNI)
        let mut cnes_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let cnes = get_str_value(raw_batch, "PA_CODUNI", i).unwrap_or("0000000");
            cnes_builder.append_value(cnes);
        }
        let cnes_col: ArrayRef = Arc::new(cnes_builder.finish());

        // 4. facility_municipality (PA_UFMUN)
        let mut fac_mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "PA_UFMUN", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            fac_mun_builder.append_value(resolved);
        }
        let fac_mun_col: ArrayRef = Arc::new(fac_mun_builder.finish());

        // 5. procedure_sigtap (PA_PROC_ID)
        let mut proc_builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
        for i in 0..num_rows {
            let proc_id = get_str_value(raw_batch, "PA_PROC_ID", i).unwrap_or("0000000000");
            proc_builder.append_value(proc_id);
        }
        let proc_col: ArrayRef = Arc::new(proc_builder.finish());

        // 6. service_date (PA_CMP)
        let mut date_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dt = get_date32_value(raw_batch, "PA_CMP", i).unwrap_or(0);
            date_builder.append_value(dt);
        }
        let date_col: ArrayRef = Arc::new(date_builder.finish());

        // 7. main_diagnosis_icd10 (PA_CIDPRI)
        let mut cid_builder = StringBuilder::with_capacity(num_rows, num_rows * 5);
        for i in 0..num_rows {
            if let Some(cid) = get_str_value(raw_batch, "PA_CIDPRI", i) {
                cid_builder.append_value(cid);
            } else {
                cid_builder.append_null();
            }
        }
        let cid_col: ArrayRef = Arc::new(cid_builder.finish());

        // 8. quantity_produced (PA_QTDPRO)
        let mut qty_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let q = get_u32_value(raw_batch, "PA_QTDPRO", i).unwrap_or(1);
            qty_builder.append_value(q);
        }
        let qty_col: ArrayRef = Arc::new(qty_builder.finish());

        // 9. total_paid_amount (PA_VALPRO)
        let mut cost_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let val = get_float64_value(raw_batch, "PA_VALPRO", i).unwrap_or(0.0);
            cost_builder.append_value(val);
        }
        let cost_col: ArrayRef = Arc::new(cost_builder.finish());

        // 10. patient_sex (PA_SEXO)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows);
        for i in 0..num_rows {
            if let Some(s) = get_str_value(raw_batch, "PA_SEXO", i) {
                sex_builder.append_value(s);
            } else {
                sex_builder.append_null();
            }
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 11. patient_age_years (PA_IDADE)
        let mut age_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(age) = get_u16_value(raw_batch, "PA_IDADE", i) {
                age_builder.append_value(age);
            } else {
                age_builder.append_null();
            }
        }
        let age_col: ArrayRef = Arc::new(age_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                record_id_col,
                patient_mun_col,
                cnes_col,
                fac_mun_col,
                proc_col,
                date_col,
                cid_col,
                qty_col,
                cost_col,
                sex_col,
                age_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SiasusDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.siasus",
            display_name: "SIASUS - Sistema de Informações Ambulatoriais do SUS",
            maintaining_agency: "DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::AssistanceInfrastructure,
            temporal_resolution: "Mensal",
            spatial_resolution: "Município / Estabelecimento (CNES)",
            supported_years: 1994..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_ambulatory_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let sub_system = params
            .extra_filters
            .get("sub_system")
            .map(|s| s.as_str())
            .unwrap_or("PA"); // PA = Produção Ambulatorial, APAC = Alta Complexidade
        let year_short = params.year % 100;
        let month = params.month.unwrap_or(1);
        let filename = format!(
            "{}{}{:02}{:02}.dbc",
            sub_system.to_uppercase(),
            uf.to_uppercase(),
            year_short,
            month
        );

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIASUS/200801_/Dados/{filename}"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;
        let dbf_bytes = context.decompressor.decompress(&raw_bytes)?;

        let decoder = DbfDecoder::new();
        let raw_batch = decoder.decode_to_record_batch(&dbf_bytes)?;
        let harmonized = self.harmonize_batch(&raw_batch)?;

        Ok(vec![harmonized])
    }
}
