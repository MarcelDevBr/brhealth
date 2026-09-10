// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Banco de Preços em Saúde (BPS / CMED / Anvisa - Ministério da Saúde).

use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_f64_col, build_harmonized_ibge_col, build_str_col, build_str_opt_col, build_u32_col,
    get_date32_value, get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o Banco de Preços em Saúde (BPS / CMED) de compras públicas de medicamentos.
#[derive(Debug, Default, Clone)]
pub struct BpsDataSource;

impl BpsDataSource {
    /// Cria uma nova instância de `BpsDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` para o schema canônico de preços de medicamentos.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_drug_price_schema();

        // 1. purchase_id (NU_COMPRA ou ID_ITEM)
        let id_col: ArrayRef = {
            let mut id_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 12);
            for i in 0..num_rows {
                let id = get_str_value(raw_batch, "NU_COMPRA", i)
                    .or_else(|| get_str_value(raw_batch, "ID_ITEM", i))
                    .unwrap_or("");
                if id.is_empty() {
                    id_builder.append_value(format!("BUY_{i}"));
                } else {
                    id_builder.append_value(id);
                }
            }
            Arc::new(id_builder.finish())
        };

        // 2. procurement_date (DT_COMPRA ou DT_HOMOLOGACAO)
        let dt_col: ArrayRef = {
            let mut dt_builder = arrow::array::Date32Builder::with_capacity(num_rows);
            for i in 0..num_rows {
                let dt = get_date32_value(raw_batch, "DT_COMPRA", i)
                    .or_else(|| get_date32_value(raw_batch, "DT_HOMOLOGACAO", i))
                    .unwrap_or(0);
                dt_builder.append_value(dt);
            }
            Arc::new(dt_builder.finish())
        };

        let mun_col = build_harmonized_ibge_col(raw_batch, "CO_IBGE_COMPRADOR", "0000000", num_rows);

        // 4. active_ingredient (DS_PRINCIPIO_ATIVO ou NOME_MEDICAMENTO)
        let drug_col: ArrayRef = {
            let mut drug_builder = arrow::array::StringBuilder::with_capacity(num_rows, num_rows * 20);
            for i in 0..num_rows {
                let drug = get_str_value(raw_batch, "DS_PRINCIPIO_ATIVO", i)
                    .or_else(|| get_str_value(raw_batch, "NOME_MEDICAMENTO", i))
                    .unwrap_or("MEDICAMENTO NAO INFORMADO");
                drug_builder.append_value(drug);
            }
            Arc::new(drug_builder.finish())
        };

        let atc_col = build_str_opt_col(raw_batch, "CO_ATC", 7, num_rows);
        let form_col = build_str_col(raw_batch, "DS_FORMA_FARMACEUTICA", "COMPRIMIDO", num_rows);
        let qty_col = build_u32_col(raw_batch, "QT_ADQUIRIDA", 1, num_rows);
        let unit_price_col = build_f64_col(raw_batch, "VL_UNITARIO", 0.0, num_rows);
        let total_price_col = build_f64_col(raw_batch, "VL_TOTAL", 0.0, num_rows);

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col,
                dt_col,
                mun_col,
                drug_col,
                atc_col,
                form_col,
                qty_col,
                unit_price_col,
                total_price_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for BpsDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.bps",
            display_name: "BPS / CMED - Banco de Preços em Saúde e Fármacos",
            maintaining_agency: "Ministério da Saúde / Anvisa",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::FinancialAdministrative,
            temporal_resolution: "Mensal / Semestral",
            spatial_resolution: "Município / Ente Comprador",
            supported_years: 2000..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_drug_price_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let year = params.year;
        let filename = format!("BPS_{}.dbc", year);

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/BPS/DADOS/{filename}"
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
