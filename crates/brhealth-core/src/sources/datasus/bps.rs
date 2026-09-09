// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Banco de Preços em Saúde (BPS / CMED / Anvisa - Ministério da Saúde).

use std::sync::Arc;

use arrow::array::{ArrayRef, Date32Builder, Float64Builder, StringBuilder, UInt32Builder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_date32_value, get_float64_value, get_str_value, get_u32_value};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

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
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
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
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. procurement_date (DT_COMPRA ou DT_HOMOLOGACAO)
        let mut dt_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dt = get_date32_value(raw_batch, "DT_COMPRA", i)
                .or_else(|| get_date32_value(raw_batch, "DT_HOMOLOGACAO", i))
                .unwrap_or(0);
            dt_builder.append_value(dt);
        }
        let dt_col: ArrayRef = Arc::new(dt_builder.finish());

        // 3. buyer_jurisdiction (CO_IBGE_COMPRADOR ou UF_COMPRADOR)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CO_IBGE_COMPRADOR", i)
                .and_then(|m| harmonize_ibge_code(m).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 4. active_ingredient (DS_PRINCIPIO_ATIVO)
        let mut drug_builder = StringBuilder::with_capacity(num_rows, num_rows * 20);
        for i in 0..num_rows {
            let drug = get_str_value(raw_batch, "DS_PRINCIPIO_ATIVO", i)
                .or_else(|| get_str_value(raw_batch, "NOME_MEDICAMENTO", i))
                .unwrap_or("MEDICAMENTO NAO INFORMADO");
            drug_builder.append_value(drug);
        }
        let drug_col: ArrayRef = Arc::new(drug_builder.finish());

        // 5. atc_code (CO_ATC)
        let mut atc_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            if let Some(atc) = get_str_value(raw_batch, "CO_ATC", i) {
                atc_builder.append_value(atc);
            } else {
                atc_builder.append_null();
            }
        }
        let atc_col: ArrayRef = Arc::new(atc_builder.finish());

        // 6. dosage_form (DS_FORMA_FARMACEUTICA)
        let mut form_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let form = get_str_value(raw_batch, "DS_FORMA_FARMACEUTICA", i).unwrap_or("COMPRIMIDO");
            form_builder.append_value(form);
        }
        let form_col: ArrayRef = Arc::new(form_builder.finish());

        // 7. quantity_purchased (QT_ADQUIRIDA)
        let mut qty_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let qty = get_u32_value(raw_batch, "QT_ADQUIRIDA", i).unwrap_or(1);
            qty_builder.append_value(qty);
        }
        let qty_col: ArrayRef = Arc::new(qty_builder.finish());

        // 8. unit_price_brl (VL_UNITARIO)
        let mut unit_price_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let price = get_float64_value(raw_batch, "VL_UNITARIO", i).unwrap_or(0.0);
            unit_price_builder.append_value(price);
        }
        let unit_price_col: ArrayRef = Arc::new(unit_price_builder.finish());

        // 9. total_price_brl (VL_TOTAL)
        let mut total_price_builder = Float64Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let total = get_float64_value(raw_batch, "VL_TOTAL", i).unwrap_or(0.0);
            total_price_builder.append_value(total);
        }
        let total_price_col: ArrayRef = Arc::new(total_price_builder.finish());

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
