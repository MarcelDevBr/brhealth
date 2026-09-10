// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Sistema de Informações sobre Nascidos Vivos (SINASC - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, StringBuilder};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{
    build_constant_str_col, build_date32_col, build_harmonized_ibge_col, build_null_col,
    build_race_col, build_record_id_col, build_sex_col, build_u8_opt_col, build_u16_opt_col,
    get_str_value,
};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};

/// Adaptador SPI para o SINASC (Nascidos Vivos) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct SinascDataSource;

use crate::domain::schema::default_values::PREFIX_SINASC;

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
        let record_id_col = build_record_id_col(raw_batch, "NUMERODN", PREFIX_SINASC, num_rows);

        // 2. country_iso3 ("BRA")
        let country_col = build_constant_str_col("BRA", num_rows);

        // 3. jurisdiction_code (CODMUNRES)
        let jurisdiction_col = build_harmonized_ibge_col(raw_batch, "CODMUNRES", num_rows);

        // 4. h3_index_res8 (Nulo inicial)
        let h3_col = build_null_col(&DataType::UInt64, num_rows);

        // 5. birth_date (DTNASC)
        let birth_date_col = build_date32_col(raw_batch, "DTNASC", 0, num_rows);

        // 6. birth_weight_grams (PESO)
        let weight_col = build_u16_opt_col(raw_batch, "PESO", num_rows);

        // 7. gestational_weeks (SEMAGESTAC)
        let gest_col = build_u8_opt_col(raw_batch, "SEMAGESTAC", num_rows);

        // 8. apgar_1min (APGAR1)
        let ap1_col = build_u8_opt_col(raw_batch, "APGAR1", num_rows);

        // 9. apgar_5min (APGAR5)
        let ap5_col = build_u8_opt_col(raw_batch, "APGAR5", num_rows);

        // 10. sex (SEXO)
        let sex_col = build_sex_col(raw_batch, "SEXO", num_rows);

        // 11. race_ethnicity (RACACOR)
        let race_col = build_race_col(raw_batch, "RACACOR", num_rows);

        // 12. mother_age_years (IDADEMAE)
        let mae_col = build_u8_opt_col(raw_batch, "IDADEMAE", num_rows);

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
