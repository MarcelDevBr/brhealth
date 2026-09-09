// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do SISAGUA (Ministério da Saúde - Vigilância da Qualidade da Água para Consumo Humano).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, Date32Builder, Float32Builder, StringBuilder};
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
use crate::domain::transforms::ibge::harmonize_ibge_code;
use crate::sources::datasus::helpers::{
    get_bool_value, get_date32_value, get_float32_value, get_str_value,
};

/// Adaptador SPI para vigilância da qualidade da água (SISAGUA / Ministério da Saúde).
#[derive(Debug, Default, Clone)]
pub struct SisaguaDataSource;

impl SisaguaDataSource {
    /// Cria uma nova instância de `SisaguaDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de amostras para o schema canônico de qualidade da água.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_water_quality_schema();

        // 1. sample_id (ID_AMOSTRA ou NU_AMOSTRA)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "ID_AMOSTRA", i)
                .or_else(|| get_str_value(raw_batch, "NU_AMOSTRA", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("WATER_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. collection_date (DT_COLETA)
        let mut dt_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dt = get_date32_value(raw_batch, "DT_COLETA", i).unwrap_or(0);
            dt_builder.append_value(dt);
        }
        let dt_col: ArrayRef = Arc::new(dt_builder.finish());

        // 3. municipality_code (CO_IBGE_MUNICIPIO)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CO_IBGE_MUNICIPIO", i)
                .and_then(|m| harmonize_ibge_code(m).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 4. water_supply_system_id (CO_SISTEMA_ABASTECIMENTO)
        let mut sys_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            let sys = get_str_value(raw_batch, "CO_SISTEMA_ABASTECIMENTO", i).unwrap_or("SAA01");
            sys_builder.append_value(sys);
        }
        let sys_col: ArrayRef = Arc::new(sys_builder.finish());

        // 5. sampling_point_type (TP_PONTO_COLETA)
        let mut pt_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let pt = get_str_value(raw_batch, "TP_PONTO_COLETA", i).unwrap_or("REDE_DISTRIBUICAO");
            pt_builder.append_value(pt);
        }
        let pt_col: ArrayRef = Arc::new(pt_builder.finish());

        // 6. total_coliforms_detected (ST_COLIFORMES_TOTAIS)
        let mut col_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let c = get_bool_value(raw_batch, "ST_COLIFORMES_TOTAIS", i).unwrap_or(false);
            col_builder.append_value(c);
        }
        let col_col: ArrayRef = Arc::new(col_builder.finish());

        // 7. escherichia_coli_detected (ST_E_COLI)
        let mut ecoli_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let ec = get_bool_value(raw_batch, "ST_E_COLI", i).unwrap_or(false);
            ecoli_builder.append_value(ec);
        }
        let ecoli_col: ArrayRef = Arc::new(ecoli_builder.finish());

        // 8. free_residual_chlorine_mg_l (VL_CLORO_RESIDUAL)
        let mut cl_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(cl) = get_float32_value(raw_batch, "VL_CLORO_RESIDUAL", i) {
                cl_builder.append_value(cl);
            } else {
                cl_builder.append_null();
            }
        }
        let cl_col: ArrayRef = Arc::new(cl_builder.finish());

        // 9. turbidity_ntu (VL_TURBIDEZ)
        let mut turb_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(t) = get_float32_value(raw_batch, "VL_TURBIDEZ", i) {
                turb_builder.append_value(t);
            } else {
                turb_builder.append_null();
            }
        }
        let turb_col: ArrayRef = Arc::new(turb_builder.finish());

        // 10. fluoride_mg_l (VL_FLUORETO)
        let mut f_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(f) = get_float32_value(raw_batch, "VL_FLUORETO", i) {
                f_builder.append_value(f);
            } else {
                f_builder.append_null();
            }
        }
        let f_col: ArrayRef = Arc::new(f_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col, dt_col, mun_col, sys_col, pt_col, col_col, ecoli_col, cl_col, turb_col,
                f_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for SisaguaDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "environmental.sisagua",
            display_name: "SISAGUA - Vigilância da Qualidade da Água para Consumo Humano",
            maintaining_agency: "Ministério da Saúde / SVSA",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::EnvironmentalPlanetary,
            temporal_resolution: "Mensal",
            spatial_resolution: "Município / Ponto de Coleta (IBGE)",
            supported_years: 2007..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_water_quality_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        Ok(format!(
            "https://dados.gov.br/dados/conjuntos-dados/sisagua-vigilancia/{year}.parquet"
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
