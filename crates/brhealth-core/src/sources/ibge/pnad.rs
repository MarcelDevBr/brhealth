// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI da Pesquisa Nacional por Amostra de Domicílios Contínua (PNAD Contínua - IBGE).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, StringBuilder, UInt16Builder, UInt8Builder,
};
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
use crate::sources::datasus::helpers::{
    get_bool_value, get_float64_value, get_str_value, get_u16_value, get_u8_value,
};

/// Adaptador SPI para a PNAD Contínua do IBGE.
#[derive(Debug, Default, Clone)]
pub struct IbgePnadDataSource;

impl IbgePnadDataSource {
    /// Cria uma nova instância de `IbgePnadDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de microdados amostrais para o schema canônico da PNAD.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_socioeconomic_pnad_schema();

        // 1. survey_id (UPA + V1008 + V1014 ou gerado)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 16);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "UPA", i)
                .or_else(|| get_str_value(raw_batch, "ID_DOMICILIO", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("PNAD_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. survey_year (Ano)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "Ano", i)
                .or_else(|| get_u16_value(raw_batch, "ANO", i))
                .unwrap_or(2023);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 3. survey_quarter (Trimestre)
        let mut q_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let q = get_u8_value(raw_batch, "Trimestre", i)
                .or_else(|| get_u8_value(raw_batch, "TRIMESTRE", i))
                .unwrap_or(1);
            q_builder.append_value(q);
        }
        let q_col: ArrayRef = Arc::new(q_builder.finish());

        // 4. state_code (UF)
        let mut uf_builder = StringBuilder::with_capacity(num_rows, num_rows * 2);
        for i in 0..num_rows {
            let uf = get_str_value(raw_batch, "UF", i).unwrap_or("35");
            uf_builder.append_value(uf);
        }
        let uf_col: ArrayRef = Arc::new(uf_builder.finish());

        // 5. sample_weight (V1028 ou PESO_AMOSTRAL)
        let mut w_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let w = get_float64_value(raw_batch, "V1028", i)
                .or_else(|| get_float64_value(raw_batch, "PESO_AMOSTRAL", i))
                .unwrap_or(1.0);
            w_builder.append_value(w);
        }
        let w_col: ArrayRef = Arc::new(w_builder.finish());

        // 6. head_of_household_sex (V2007 ou SEXO_RESP)
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows);
        for i in 0..num_rows {
            let s = get_str_value(raw_batch, "V2007", i)
                .or_else(|| get_str_value(raw_batch, "SEXO_RESP", i))
                .unwrap_or("M");
            sex_builder.append_value(s);
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 7. per_capita_household_income (VD5008 ou RENDA_PERCAPITA)
        let mut inc_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(inc) = get_float64_value(raw_batch, "VD5008", i)
                .or_else(|| get_float64_value(raw_batch, "RENDA_PERCAPITA", i))
            {
                inc_builder.append_value(inc);
            } else {
                inc_builder.append_null();
            }
        }
        let inc_col: ArrayRef = Arc::new(inc_builder.finish());

        // 8. has_private_health_insurance (V4001 ou PLANO_SAUDE)
        let mut plan_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let has_plan = get_bool_value(raw_batch, "V4001", i)
                .or_else(|| get_bool_value(raw_batch, "PLANO_SAUDE", i))
                .unwrap_or(false);
            plan_builder.append_value(has_plan);
        }
        let plan_col: ArrayRef = Arc::new(plan_builder.finish());

        // 9. education_level_years (VD3005 ou ANOS_ESTUDO)
        let mut edu_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(edu) = get_u8_value(raw_batch, "VD3005", i)
                .or_else(|| get_u8_value(raw_batch, "ANOS_ESTUDO", i))
            {
                edu_builder.append_value(edu);
            } else {
                edu_builder.append_null();
            }
        }
        let edu_col: ArrayRef = Arc::new(edu_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, yr_col, q_col, uf_col, w_col, sex_col, inc_col, plan_col, edu_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for IbgePnadDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "ibge.pnad",
            display_name: "IBGE PNAD Contínua - Pesquisa Nacional por Amostra de Domicílios",
            maintaining_agency: "Instituto Brasileiro de Geografia e Estatística (IBGE)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Trimestral",
            spatial_resolution: "Unidade Federativa / Grande Região",
            supported_years: 2012..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_socioeconomic_pnad_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        let quarter = params.month.map(|m| (m.saturating_sub(1) / 3) + 1).unwrap_or(1);
        let filename = format!("PNADC_{:04}_trimestre_{:02}.parquet", year, quarter);

        Ok(format!(
            "https://ftp.ibge.gov.br/Trabalho_e_Rendimento/Pesquisa_Nacional_por_Amostra_de_Domicilios_continua/Trimestral/Microdados/{year}/{filename}"
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
