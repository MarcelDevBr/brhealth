// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações sobre Mortalidade (SIM - DATASUS).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, StringBuilder, UInt16Builder, UInt64Builder,
};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_date32_value, get_str_value};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o SIM (Mortalidade Geral) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SimDataSource;

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
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "NUMERODO", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("SIM_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let record_id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. country_iso3 ("BRA")
        let mut country_builder = StringBuilder::with_capacity(num_rows, num_rows * 3);
        for _ in 0..num_rows {
            country_builder.append_value("BRA");
        }
        let country_col: ArrayRef = Arc::new(country_builder.finish());

        // 3. jurisdiction_code (CODMUNRES harmonizado para 7 dígitos)
        let mut juris_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CODMUNRES", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            juris_builder.append_value(resolved);
        }
        let jurisdiction_col: ArrayRef = Arc::new(juris_builder.finish());

        // 4. h3_index_res8 (Nulo por padrão até join com centróides ou endereços)
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 5. event_date (DTOBITO)
        let mut date_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let date_val = get_date32_value(raw_batch, "DTOBITO", i).unwrap_or(0);
            date_builder.append_value(date_val);
        }
        let event_date_col: ArrayRef = Arc::new(date_builder.finish());

        // 6. underlying_cause_icd10 (CAUSABAS)
        let mut causa_builder = StringBuilder::with_capacity(num_rows, num_rows * 5);
        for i in 0..num_rows {
            let causa = get_str_value(raw_batch, "CAUSABAS", i).unwrap_or("R99");
            causa_builder.append_value(causa);
        }
        let causa_col: ArrayRef = Arc::new(causa_builder.finish());

        // 7. underlying_cause_icd11 (Nulo nesta etapa)
        let mut causa11_builder = StringBuilder::with_capacity(num_rows, 0);
        for _ in 0..num_rows {
            causa11_builder.append_null();
        }
        let causa11_col: ArrayRef = Arc::new(causa11_builder.finish());

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
        let mut sex_builder = StringBuilder::with_capacity(num_rows, num_rows * 2);
        for i in 0..num_rows {
            let sex_str = match get_str_value(raw_batch, "SEXO", i) {
                Some("1" | "M") => "M",
                Some("2" | "F") => "F",
                _ => "U",
            };
            sex_builder.append_value(sex_str);
        }
        let sex_col: ArrayRef = Arc::new(sex_builder.finish());

        // 10. race_ethnicity (RACACOR)
        let mut race_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            let race_str = match get_str_value(raw_batch, "RACACOR", i) {
                Some("1") => Some("Branca"),
                Some("2") => Some("Preta"),
                Some("3") => Some("Amarela"),
                Some("4") => Some("Parda"),
                Some("5") => Some("Indígena"),
                _ => None,
            };
            if let Some(r) = race_str {
                race_builder.append_value(r);
            } else {
                race_builder.append_null();
            }
        }
        let race_col: ArrayRef = Arc::new(race_builder.finish());

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
