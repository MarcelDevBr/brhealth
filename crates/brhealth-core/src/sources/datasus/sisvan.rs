// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Vigilância Alimentar e Nutricional (SISVAN - DATASUS / SAPS).

use std::sync::Arc;

use arrow::array::{ArrayRef, Float32Builder};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_date32_col, build_null_col, build_record_id_col, build_sex_col, build_u16_col,
    get_float32_value, get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SISVAN (Nutrição e Antropometria) do Ministério da Saúde.
#[derive(Debug, Default, Clone)]
pub struct SisvanDataSource;

impl SisvanDataSource {
    /// Cria uma nova instância de `SisvanDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` para o schema canônico de vigilância nutricional.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_nutritional_surveillance_schema();

        let id_col = build_record_id_col(raw_batch, "ID_ACOMP", "NUTRI", num_rows);
        let dt_col = build_date32_col(raw_batch, "DT_ACOMP", 0, num_rows);

        // patient_municipality (CO_MUNICIPIO_IBGE)
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

        let h3_col = build_null_col(&DataType::UInt64, num_rows);
        let age_col = build_u16_col(raw_batch, "NU_IDADE_MESES", 0, num_rows);
        let sex_col = build_sex_col(raw_batch, "SG_SEXO", num_rows);

        // weight_kg (NU_PESO)
        let wt_col: ArrayRef = {
            let mut wt_builder = Float32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let w = get_float32_value(raw_batch, "NU_PESO", i).unwrap_or(0.0);
                wt_builder.append_value(w);
            }
            Arc::new(wt_builder.finish())
        };

        // height_cm (NU_ALTURA)
        let ht_col: ArrayRef = {
            let mut ht_builder = Float32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let h = get_float32_value(raw_batch, "NU_ALTURA", i).unwrap_or(0.0);
                ht_builder.append_value(h);
            }
            Arc::new(ht_builder.finish())
        };

        // bmi (calculado ou extraído de NU_IMC)
        let bmi_col: ArrayRef = {
            let mut bmi_builder = Float32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let bmi = get_float32_value(raw_batch, "NU_IMC", i).or_else(|| {
                    let w = get_float32_value(raw_batch, "NU_PESO", i)?;
                    let h_cm = get_float32_value(raw_batch, "NU_ALTURA", i)?;
                    if h_cm > 0.0 {
                        let h_m = h_cm / 100.0;
                        Some(w / (h_m * h_m))
                    } else {
                        None
                    }
                });
                if let Some(b) = bmi {
                    bmi_builder.append_value(b);
                } else {
                    bmi_builder.append_null();
                }
            }
            Arc::new(bmi_builder.finish())
        };

        // who_growth_classification (DS_FAIXA_IMC ou CLAS_ESTADO_NUTRICIONAL)
        let class_col: ArrayRef = {
            let mut class_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 16);
            for i in 0..num_rows {
                if let Some(c) = get_str_value(raw_batch, "DS_FAIXA_IMC", i)
                    .or_else(|| get_str_value(raw_batch, "CLAS_ESTADO_NUTRICIONAL", i))
                {
                    class_builder.append_value(c);
                } else {
                    class_builder.append_null();
                }
            }
            Arc::new(class_builder.finish())
        };

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, dt_col, mun_col, h3_col, age_col, sex_col, wt_col, ht_col, bmi_col,
                class_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SisvanDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sisvan",
            display_name: "SISVAN - Sistema de Vigilância Alimentar e Nutricional",
            maintaining_agency: "Ministério da Saúde / SAPS",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Mensal / Anual",
            spatial_resolution: "Município / UBS (IBGE)",
            supported_years: 2008..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_nutritional_surveillance_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        let filename = format!("VAN_{}_{}.dbc", uf.to_uppercase(), year);

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SISVAN/DADOS/{filename}"
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
