// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informação de Agravos de Notificação (SINAN - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, Date32Builder, StringBuilder, UInt16Builder, UInt64Builder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_date32_value, get_str_value, get_u16_value};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SINAN (Notificações de Agravos) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SinanDataSource;

impl SinanDataSource {
    /// Cria uma nova instância de `SinanDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de agravos.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_notifiable_disease_schema();

        // 1. notification_id (NU_NOTIFIC)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "NU_NOTIFIC", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("NOTIF_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let notification_id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. disease_code (ID_AGRAVO)
        let mut disease_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            let d = get_str_value(raw_batch, "ID_AGRAVO", i).unwrap_or("A90");
            disease_builder.append_value(d);
        }
        let disease_col: ArrayRef = Arc::new(disease_builder.finish());

        // 3. notification_date (DT_NOTIFIC)
        let mut notif_date_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dt = get_date32_value(raw_batch, "DT_NOTIFIC", i).unwrap_or(0);
            notif_date_builder.append_value(dt);
        }
        let notif_date_col: ArrayRef = Arc::new(notif_date_builder.finish());

        // 4. symptom_onset_date (DT_SIN_PRI)
        let mut onset_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(dt) = get_date32_value(raw_batch, "DT_SIN_PRI", i) {
                onset_builder.append_value(dt);
            } else {
                onset_builder.append_null();
            }
        }
        let onset_col: ArrayRef = Arc::new(onset_builder.finish());

        // 5. patient_municipality (ID_MN_RESI)
        let mut res_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "ID_MN_RESI", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            res_builder.append_value(resolved);
        }
        let patient_mun_col: ArrayRef = Arc::new(res_builder.finish());

        // 6. notification_municipality (ID_MUNICIP)
        let mut notif_mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "ID_MUNICIP", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            notif_mun_builder.append_value(resolved);
        }
        let notif_mun_col: ArrayRef = Arc::new(notif_mun_builder.finish());

        // 7. h3_index_res8
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 8. age_years (NU_IDADE_N)
        let mut age_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(age) = get_u16_value(raw_batch, "NU_IDADE_N", i) {
                age_builder.append_value(age);
            } else {
                age_builder.append_null();
            }
        }
        let age_col: ArrayRef = Arc::new(age_builder.finish());

        // 9. sex (CS_SEXO)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows);
        for i in 0..num_rows {
            let s = get_str_value(raw_batch, "CS_SEXO", i).unwrap_or("U");
            sex_builder.append_value(s);
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 10. diagnostic_criterion (CRITERIO)
        let mut crit_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            if let Some(crit) = get_str_value(raw_batch, "CRITERIO", i) {
                crit_builder.append_value(crit);
            } else {
                crit_builder.append_null();
            }
        }
        let crit_col: ArrayRef = Arc::new(crit_builder.finish());

        // 11. case_classification (CLASSI_FIN)
        let mut class_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            if let Some(c) = get_str_value(raw_batch, "CLASSI_FIN", i) {
                class_builder.append_value(c);
            } else {
                class_builder.append_null();
            }
        }
        let class_col: ArrayRef = Arc::new(class_builder.finish());

        // 12. closure_outcome (EVOLUCAO)
        let mut outcome_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            if let Some(e) = get_str_value(raw_batch, "EVOLUCAO", i) {
                outcome_builder.append_value(e);
            } else {
                outcome_builder.append_null();
            }
        }
        let outcome_col: ArrayRef = Arc::new(outcome_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                notification_id_col,
                disease_col,
                notif_date_col,
                onset_col,
                patient_mun_col,
                notif_mun_col,
                h3_col,
                age_col,
                sex_col,
                crit_col,
                class_col,
                outcome_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SinanDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sinan",
            display_name: "SINAN - Sistema de Informação de Agravos de Notificação",
            maintaining_agency: "DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::ClinicalMorbidity,
            temporal_resolution: "Diária / Semanal / Mensal",
            spatial_resolution: "Município / UF (IBGE)",
            supported_years: 1998..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_notifiable_disease_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let disease = params
            .extra_filters
            .get("disease")
            .map(|s| s.as_str())
            .unwrap_or("DENG");
        let year_short = params.year % 100;
        let filename = format!("{}{}{:02}.dbc", disease.to_uppercase(), uf.to_uppercase(), year_short);

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SINAN/DADOS/PRELIM/{filename}"
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
