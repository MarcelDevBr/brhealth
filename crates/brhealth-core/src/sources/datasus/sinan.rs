// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informação de Agravos de Notificação (SINAN - DATASUS).

use std::sync::Arc;

use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_date32_col, build_date32_opt_col, build_harmonized_ibge_col, build_null_col,
    build_record_id_col, build_str_col, build_str_opt_col, build_u16_opt_col,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o SINAN (Notificações de Agravos) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SinanDataSource;

use crate::domain::schema::default_values::PREFIX_SINAN;

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
        let notification_id_col =
            build_record_id_col(raw_batch, "NU_NOTIFIC", PREFIX_SINAN, num_rows);

        // 2. disease_code (ID_AGRAVO)
        let disease_col = build_str_col(raw_batch, "ID_AGRAVO", "A90", num_rows);

        // 3. notification_date (DT_NOTIFIC)
        let notif_date_col = build_date32_col(raw_batch, "DT_NOTIFIC", 0, num_rows);

        // 4. symptom_onset_date (DT_SIN_PRI)
        let onset_col = build_date32_opt_col(raw_batch, "DT_SIN_PRI", num_rows);

        // 5. patient_municipality (ID_MN_RESI)
        let patient_mun_col = build_harmonized_ibge_col(raw_batch, "ID_MN_RESI", num_rows);

        // 6. notification_municipality (ID_MUNICIP)
        let notif_mun_col = build_harmonized_ibge_col(raw_batch, "ID_MUNICIP", num_rows);

        // 7. h3_index_res8
        let h3_col = build_null_col(&DataType::UInt64, num_rows);

        // 8. age_years (NU_IDADE_N)
        let age_col = build_u16_opt_col(raw_batch, "NU_IDADE_N", num_rows);

        // 9. sex (CS_SEXO)
        let sex_col = build_str_col(raw_batch, "CS_SEXO", "U", num_rows);

        // 10. diagnostic_criterion (CRITERIO)
        let crit_col = build_str_opt_col(raw_batch, "CRITERIO", 4, num_rows);

        // 11. case_classification (CLASSI_FIN)
        let class_col = build_str_opt_col(raw_batch, "CLASSI_FIN", 4, num_rows);

        // 12. closure_outcome (EVOLUCAO)
        let outcome_col = build_str_opt_col(raw_batch, "EVOLUCAO", 4, num_rows);

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
        let filename = format!(
            "{}{}{:02}.dbc",
            disease.to_uppercase(),
            uf.to_uppercase(),
            year_short
        );

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
