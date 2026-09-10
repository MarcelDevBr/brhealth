// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informação do Câncer (SISCAN / SISCOLO / SISMAMA - INCA / DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_str_opt_col, get_bool_value, get_date32_value, get_str_value, get_u8_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SISCAN (Rastreamento de Câncer de Mama e Colo Uterino).
#[derive(Debug, Default, Clone)]
pub struct SiscanDataSource;

impl SiscanDataSource {
    /// Cria uma nova instância de `SiscanDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` para o schema canônico de rastreamento oncológico.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_cancer_screening_schema();

        // 1. exam_id (CO_EXAME ou NU_PEDIDO)
        let id_col: ArrayRef = {
            let mut id_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 12);
            for i in 0..num_rows {
                let id = get_str_value(raw_batch, "CO_EXAME", i)
                    .or_else(|| get_str_value(raw_batch, "NU_PEDIDO", i))
                    .unwrap_or("");
                if id.is_empty() {
                    id_builder.append_value(format!("EXAM_{i}"));
                } else {
                    id_builder.append_value(id);
                }
            }
            Arc::new(id_builder.finish())
        };

        // 2. cancer_type (TP_EXAME: "MAMO" -> "BREAST", "CITO" -> "CERVICAL")
        let type_col: ArrayRef = {
            let mut type_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 8);
            for i in 0..num_rows {
                let raw_type = get_str_value(raw_batch, "TP_EXAME", i).unwrap_or("MAMO");
                let normed = if raw_type.contains("CITO") || raw_type.contains("COLO") {
                    "CERVICAL"
                } else {
                    "BREAST"
                };
                type_builder.append_value(normed);
            }
            Arc::new(type_builder.finish())
        };

        // 3. exam_date (DT_EXAME ou DT_COLETA)
        let dt_col: ArrayRef = {
            let mut dt_builder = arrow::array::Date32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let dt = get_date32_value(raw_batch, "DT_EXAME", i)
                    .or_else(|| get_date32_value(raw_batch, "DT_COLETA", i))
                    .unwrap_or(0);
                dt_builder.append_value(dt);
            }
            Arc::new(dt_builder.finish())
        };

        // 4. patient_municipality (CO_MUNICIPIO_IBGE ou CODMUNRES)
        let mun_col: ArrayRef = {
            let mut mun_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 7);
            for i in 0..num_rows {
                let resolved = get_str_value(raw_batch, "CO_MUNICIPIO_IBGE", i)
                    .or_else(|| get_str_value(raw_batch, "CODMUNRES", i))
                    .and_then(|m| harmonize_ibge_code(m).ok())
                    .unwrap_or_else(|| "0000000".to_string());
                mun_builder.append_value(resolved);
            }
            Arc::new(mun_builder.finish())
        };

        // 5. patient_age_years (NU_IDADE)
        let age_col: ArrayRef = {
            let mut age_builder = arrow::array::UInt8Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let age = get_u8_value(raw_batch, "NU_IDADE", i).unwrap_or(50);
                age_builder.append_value(age);
            }
            Arc::new(age_builder.finish())
        };

        let ind_col = build_str_opt_col(raw_batch, "DS_INDICACAO_CLINICA", 16, num_rows);

        // 7. diagnostic_result (DS_RESULTADO_DIAGNOSTICO ou DS_CONCLUSAO)
        let res_col: ArrayRef = {
            let mut res_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 20);
            for i in 0..num_rows {
                let res = get_str_value(raw_batch, "DS_RESULTADO_DIAGNOSTICO", i)
                    .or_else(|| get_str_value(raw_batch, "DS_CONCLUSAO", i))
                    .unwrap_or("BI-RADS 1 / NORMAL");
                res_builder.append_value(res);
            }
            Arc::new(res_builder.finish())
        };

        // 8. biopsy_recommended (ST_RECOMENDA_BIOPSIA)
        let bio_col: ArrayRef = {
            let mut bio_builder = BooleanBuilder::with_capacity(num_rows);
            for i in 0..num_rows {
                let bio = get_bool_value(raw_batch, "ST_RECOMENDA_BIOPSIA", i).unwrap_or(false);
                bio_builder.append_value(bio);
            }
            Arc::new(bio_builder.finish())
        };

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, type_col, dt_col, mun_col, age_col, ind_col, res_col, bio_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SiscanDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.siscan",
            display_name: "SISCAN / SISMAMA / SISCOLO - Sistema de Informação do Câncer",
            maintaining_agency: "INCA / DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::ClinicalMorbidity,
            temporal_resolution: "Mensal",
            spatial_resolution: "Município / Estabelecimento (IBGE)",
            supported_years: 2013..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_cancer_screening_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let exam = params
            .extra_filters
            .get("exam_type")
            .map(|s| s.as_str())
            .unwrap_or("MAM"); // MAM = Mamografia, CIT = Citopatológico
        let year_short = params.year % 100;
        let filename = format!(
            "{}{}{:02}.dbc",
            exam.to_uppercase(),
            uf.to_uppercase(),
            year_short
        );

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SISCAN/DADOS/{filename}"
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
