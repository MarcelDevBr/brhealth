// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações sobre Nascidos Vivos (SINASC - DATASUS).

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, StringBuilder, UInt8Builder, UInt16Builder,
    UInt64Builder,
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

/// Adaptador SPI para o SINASC (Nascidos Vivos) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SinascDataSource;

impl SinascDataSource {
    /// Cria uma nova instância de `SinascDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de nascidos vivos.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_birth_schema();

        // 1. record_id (NUMERODN)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "NUMERODN", i).unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("DN_{i}"));
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

        // 3. jurisdiction_code (CODMUNRES)
        let mut juris_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CODMUNRES", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            juris_builder.append_value(resolved);
        }
        let jurisdiction_col: ArrayRef = Arc::new(juris_builder.finish());

        // 4. h3_index_res8 (Nulo inicial)
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 5. birth_date (DTNASC)
        let mut date_builder = Date32Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let date_val = get_date32_value(raw_batch, "DTNASC", i).unwrap_or(0);
            date_builder.append_value(date_val);
        }
        let birth_date_col: ArrayRef = Arc::new(date_builder.finish());

        // 6. birth_weight_grams (PESO)
        let mut weight_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let weight_val =
                get_str_value(raw_batch, "PESO", i).and_then(|s| s.parse::<u16>().ok());
            if let Some(w) = weight_val {
                weight_builder.append_value(w);
            } else {
                weight_builder.append_null();
            }
        }
        let weight_col: ArrayRef = Arc::new(weight_builder.finish());

        // 7. gestational_weeks (SEMAGESTAC)
        let mut gest_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let sema_val =
                get_str_value(raw_batch, "SEMAGESTAC", i).and_then(|s| s.parse::<u8>().ok());
            if let Some(s) = sema_val {
                gest_builder.append_value(s);
            } else {
                gest_builder.append_null();
            }
        }
        let gest_col: ArrayRef = Arc::new(gest_builder.finish());

        // 8. apgar_1min (APGAR1)
        let mut ap1_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let ap1_val = get_str_value(raw_batch, "APGAR1", i).and_then(|s| s.parse::<u8>().ok());
            if let Some(v) = ap1_val {
                ap1_builder.append_value(v);
            } else {
                ap1_builder.append_null();
            }
        }
        let ap1_col: ArrayRef = Arc::new(ap1_builder.finish());

        // 9. apgar_5min (APGAR5)
        let mut ap5_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let ap5_val = get_str_value(raw_batch, "APGAR5", i).and_then(|s| s.parse::<u8>().ok());
            if let Some(v) = ap5_val {
                ap5_builder.append_value(v);
            } else {
                ap5_builder.append_null();
            }
        }
        let ap5_col: ArrayRef = Arc::new(ap5_builder.finish());

        // 10. sex (SEXO)
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

        // 11. race_ethnicity (RACACOR)
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

        // 12. mother_age_years (IDADEMAE)
        let mut mae_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let mae_val =
                get_str_value(raw_batch, "IDADEMAE", i).and_then(|s| s.parse::<u8>().ok());
            if let Some(v) = mae_val {
                mae_builder.append_value(v);
            } else {
                mae_builder.append_null();
            }
        }
        let mae_col: ArrayRef = Arc::new(mae_builder.finish());

        // 13. delivery_type (PARTO: 1="vaginal", 2="cesarean")
        let mut parto_builder = StringBuilder::with_capacity(num_rows, num_rows * 8);
        for i in 0..num_rows {
            let parto_str = match get_str_value(raw_batch, "PARTO", i) {
                Some("1") => Some("Vaginal"),
                Some("2") => Some("Cesáreo"),
                _ => None,
            };
            if let Some(p) = parto_str {
                parto_builder.append_value(p);
            } else {
                parto_builder.append_null();
            }
        }
        let parto_col: ArrayRef = Arc::new(parto_builder.finish());

        // 14. congenital_anomaly (IDANOMAL: 1=Sim, 2=Não)
        let mut anom_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let is_anom = get_str_value(raw_batch, "IDANOMAL", i) == Some("1");
            anom_builder.append_value(is_anom);
        }
        let anom_col: ArrayRef = Arc::new(anom_builder.finish());

        let columns = vec![
            record_id_col,
            country_col,
            jurisdiction_col,
            h3_col,
            birth_date_col,
            weight_col,
            gest_col,
            ap1_col,
            ap5_col,
            sex_col,
            race_col,
            mae_col,
            parto_col,
            anom_col,
        ];

        RecordBatch::try_new(target_schema, columns).map_err(|e| {
            PortError::TabularDecodeError(format!(
                "Falha ao gerar RecordBatch canônico de nascidos vivos: {e}"
            ))
        })
    }
}

#[async_trait]
impl HealthDataSourceSPI for SinascDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.sinasc",
            display_name: "Sistema de Informações sobre Nascidos Vivos (SINASC/DATASUS)",
            maintaining_agency: "Ministério da Saúde (Brasil)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".to_string(),
            },
            category: SourceCategory::VitalStatistics,
            temporal_resolution: "Diária / Anual",
            spatial_resolution: "Municipal (IBGE 7 dígitos)",
            supported_years: 1994..=2024,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_birth_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/SINASC/1996_/Dados/DNRES/DN{uf}{year}.dbc"
        ))
    }

    async fn fetch_and_decode(
        &self,
        params: &DataQueryParams,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        let locator = self.resolve_locator(params)?;
        let raw_bytes = context.transport.fetch_bytes(&locator).await?;

        let decompressed_dbf = context.decompressor.decompress(&raw_bytes)?;

        let dbf_decoder = DbfDecoder::new();
        let raw_batch = dbf_decoder.decode_to_record_batch(&decompressed_dbf)?;

        let canonical_batch = self.harmonize_batch(&raw_batch)?;

        Ok(vec![canonical_batch])
    }
}
