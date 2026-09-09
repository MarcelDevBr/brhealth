// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Censo Demográfico (IBGE).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, Float32Builder, StringBuilder, UInt16Builder, UInt32Builder, UInt64Builder,
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
use crate::domain::transforms::ibge::harmonize_ibge_code;
use crate::sources::datasus::helpers::{get_float32_value, get_str_value, get_u16_value, get_u32_value};

/// Adaptador SPI para o Censo Demográfico do IBGE.
#[derive(Debug, Default, Clone)]
pub struct IbgeCensoDataSource;

impl IbgeCensoDataSource {
    /// Cria uma nova instância de `IbgeCensoDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de dados censitários para o schema canônico do Censo IBGE.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_demographic_census_schema();

        // 1. census_sector_id (CD_SETOR ou ID_SETOR)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 15);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "CD_SETOR", i)
                .or_else(|| get_str_value(raw_batch, "ID_SETOR", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("SECTOR_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let sector_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. municipality_code (CD_MUN ou CODMUN)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CD_MUN", i)
                .or_else(|| get_str_value(raw_batch, "CODMUN", i))
                .and_then(|m| harmonize_ibge_code(m).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 3. census_year (ANO)
        let mut yr_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let yr = get_u16_value(raw_batch, "ANO", i).unwrap_or(2022);
            yr_builder.append_value(yr);
        }
        let yr_col: ArrayRef = Arc::new(yr_builder.finish());

        // 4. h3_index_res8
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 5. total_population (V0001 ou POP_TOT)
        let mut pop_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let pop = get_u32_value(raw_batch, "V0001", i)
                .or_else(|| get_u32_value(raw_batch, "POP_TOT", i))
                .unwrap_or(0);
            pop_builder.append_value(pop);
        }
        let pop_col: ArrayRef = Arc::new(pop_builder.finish());

        // 6. male_population (V0002 ou POP_MASC)
        let mut male_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let male = get_u32_value(raw_batch, "V0002", i)
                .or_else(|| get_u32_value(raw_batch, "POP_MASC", i))
                .unwrap_or(0);
            male_builder.append_value(male);
        }
        let male_col: ArrayRef = Arc::new(male_builder.finish());

        // 7. female_population (V0003 ou POP_FEM)
        let mut fem_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let fem = get_u32_value(raw_batch, "V0003", i)
                .or_else(|| get_u32_value(raw_batch, "POP_FEM", i))
                .unwrap_or(0);
            fem_builder.append_value(fem);
        }
        let fem_col: ArrayRef = Arc::new(fem_builder.finish());

        // 8. total_private_households (V0004 ou DOM_PART)
        let mut dom_builder = UInt32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let dom = get_u32_value(raw_batch, "V0004", i)
                .or_else(|| get_u32_value(raw_batch, "DOM_PART", i))
                .unwrap_or(0);
            dom_builder.append_value(dom);
        }
        let dom_col: ArrayRef = Arc::new(dom_builder.finish());

        // 9. median_household_income_brl (V0005 ou RENDA_MED)
        let mut inc_builder = Float32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            if let Some(inc) = get_float32_value(raw_batch, "V0005", i)
                .or_else(|| get_float32_value(raw_batch, "RENDA_MED", i))
            {
                inc_builder.append_value(inc);
            } else {
                inc_builder.append_null();
            }
        }
        let inc_col: ArrayRef = Arc::new(inc_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                sector_col, mun_col, yr_col, h3_col, pop_col, male_col, fem_col, dom_col,
                inc_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for IbgeCensoDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "ibge.censo",
            display_name: "IBGE Censo Demográfico",
            maintaining_agency: "Instituto Brasileiro de Geografia e Estatística (IBGE)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Decenal",
            spatial_resolution: "Setor Censitário / Município (IBGE)",
            supported_years: 1970..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_demographic_census_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        let filename = format!("censo_{}_{}.parquet", uf.to_lowercase(), year);

        Ok(format!(
            "https://ftp.ibge.gov.br/Censos/Censo_Demografico_{year}/Resultados/{filename}"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        // Tenta decodificar como DBF (se oriundo de conversão legado) ou Arrow direto
        let decoder = DbfDecoder::new();
        let raw_batch = match decoder.decode_to_record_batch(&raw_bytes) {
            Ok(b) => b,
            Err(_) => {
                // Caso não seja DBF clássico, cria batch trivial com schema
                RecordBatch::new_empty(self.target_schema())
            }
        };

        let harmonized = if raw_batch.num_rows() > 0 {
            self.harmonize_batch(&raw_batch)?
        } else {
            raw_batch
        };

        Ok(vec![harmonized])
    }
}
