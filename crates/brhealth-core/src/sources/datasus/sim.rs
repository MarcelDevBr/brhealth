// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações sobre Mortalidade (SIM - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, UInt16Builder};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_constant_str_col, build_date32_col, build_harmonized_ibge_col, build_null_col,
    build_race_col, build_record_id_col, build_sex_col, build_str_col, get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o SIM (Mortalidade Geral) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SimDataSource;

use crate::domain::schema::default_values::PREFIX_SIM;

impl SimDataSource {
    /// Cria uma nova instância de `SimDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de mortalidade.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_mortality_schema();

        // 1. record_id (NUMERODO ou índice sequencial)
        let record_id_col = build_record_id_col(raw_batch, "NUMERODO", PREFIX_SIM, num_rows);

        // 2. country_iso3 ("BRA")
        let country_col = build_constant_str_col("BRA", num_rows);

        // 3. jurisdiction_code (CODMUNRES harmonizado para 7 dígitos)
        let jurisdiction_col = build_harmonized_ibge_col(raw_batch, "CODMUNRES", num_rows);

        // 4. h3_index_res8 (Nulo por padrão até join com centróides ou endereços)
        let h3_col = build_null_col(&DataType::UInt64, num_rows);

        // 5. event_date (DTOBITO)
        let event_date_col = build_date32_col(raw_batch, "DTOBITO", 0, num_rows);

        // 6. underlying_cause_icd10 (CAUSABAS)
        let causa_col = build_str_col(raw_batch, "CAUSABAS", "R99", num_rows);

        // 7. underlying_cause_icd11 (Nulo nesta etapa)
        let causa11_col = build_null_col(&DataType::Utf8, num_rows);

        // 8. age_years (IDADE calculada do SIM)
        let mut age_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let age_val = get_str_value(raw_batch, "IDADE", i).and_then(|raw_idade| {
                if raw_idade.len() == 3 {
                    let unit = &raw_idade[0..1];
                    let value: u16 = raw_idade[1..3].parse().unwrap_or(0);
                    match unit {
                        "4" => Some(value),
                        "5" => Some(100 + value),
                        _ => Some(0),
                    }
                } else {
                    None
                }
            });
            if let Some(a) = age_val {
                age_builder.append_value(a);
            } else {
                age_builder.append_null();
            }
        }
        let age_col: ArrayRef = Arc::new(age_builder.finish());

        // 9. sex (SEXO: 1="M", 2="F", 0="I")
        let sex_col = build_sex_col(raw_batch, "SEXO", num_rows);

        // 10. race_ethnicity (RACACOR)
        let race_col = build_race_col(raw_batch, "RACACOR", num_rows);

        // 11. maternal_death (OBITOMATER)
        let mut mat_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let is_mat = get_str_value(raw_batch, "OBITOMATER", i) == Some("1");
            mat_builder.append_value(is_mat);
        }
        let mat_col: ArrayRef = Arc::new(mat_builder.finish());

        let columns = vec![
            record_id_col,
            country_col,
            jurisdiction_col,
            h3_col,
            event_date_col,
            causa_col,
            causa11_col,
            age_col,
            sex_col,
            race_col,
            mat_col,
        ];

        RecordBatch::try_new(target_schema, columns).map_err(|e| {
            PortError::TabularDecodeError(format!(
                "Falha ao gerar RecordBatch canônico de mortalidade: {e}"
            ))
        })
    }
}

#[async_trait]
impl HealthDataSourceSPI for SimDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sim",
            display_name: "Sistema de Informações sobre Mortalidade (SIM/DATASUS)",
            maintaining_agency: "Ministério da Saúde (Brasil)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            category: SourceCategory::VitalStatistics,
            temporal_resolution: "Diária / Anual",
            spatial_resolution: "Municipal (IBGE 7 dígitos)",
            supported_years: 1979..=2024,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_mortality_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"
        ))
    }

    fn mirror_uris(&self, params: &DataQueryParams) -> Vec<String> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        vec![
            format!(
                "https://datasus.saude.gov.br/transferencia-download-de-arquivos/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"
            ),
            format!(
                "ftp://ftp2.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DO{uf}{year}.dbc"
            ),
        ]
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        // Descompressão do arquivo .dbc
        let decompressed_dbf = context.decompressor.decompress(&raw_bytes)?;

        // Decodificação DBF para RecordBatch bruto
        let dbf_decoder = DbfDecoder::new();
        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed_dbf)?;

        // Harmonização para o schema canônico
        let canonical_batch = self.harmonize_batch(&raw_batch)?;

        Ok(vec![canonical_batch])
    }
}
