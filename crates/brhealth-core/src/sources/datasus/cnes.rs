// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Cadastro Nacional de Estabelecimentos de Saúde (CNES - DATASUS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, StringBuilder, UInt16Builder, UInt64Builder};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use super::helpers::{get_bool_value, get_str_value, get_u16_value};
use crate::decoders::dbf::DbfDecoder;
use crate::domain::ports::outbound::PortError;
use crate::domain::schema::CanonicalSchemas;
use crate::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceCategory, SourceExecutionContext,
    SourceMetadata,
};
use crate::domain::transforms::ibge::harmonize_ibge_code;

/// Adaptador SPI para o CNES (Estabelecimentos e Recursos Assistenciais) do DATASUS.
#[derive(Debug, Default, Clone)]
pub struct CnesDataSource;

impl CnesDataSource {
    /// Cria uma nova instância de `CnesDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o `RecordBatch` extraído do DBF bruto para o schema canônico de estabelecimentos.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_health_facility_schema();

        // 1. cnes_id (CNES)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "CNES", i).unwrap_or("0000000");
            id_builder.append_value(id);
        }
        let cnes_id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. facility_name (NOMEFANT ou RAZAOSOC)
        let mut name_builder = StringBuilder::with_capacity(num_rows, num_rows * 30);
        for i in 0..num_rows {
            let name = get_str_value(raw_batch, "NOMEFANT", i)
                .or_else(|| get_str_value(raw_batch, "RAZAOSOC", i))
                .unwrap_or("ESTABELECIMENTO DE SAUDE");
            name_builder.append_value(name);
        }
        let name_col: ArrayRef = Arc::new(name_builder.finish());

        // 3. jurisdiction_code (CODUFMUN)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CODUFMUN", i)
                .and_then(|raw_mun| harmonize_ibge_code(raw_mun).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 4. h3_index_res8
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 5. management_type (TPGESTAO)
        let mut mgmt_builder = StringBuilder::with_capacity(num_rows, num_rows * 2);
        for i in 0..num_rows {
            let mgmt = get_str_value(raw_batch, "TPGESTAO", i).unwrap_or("M");
            mgmt_builder.append_value(mgmt);
        }
        let mgmt_col: ArrayRef = Arc::new(mgmt_builder.finish());

        // 6. facility_type_code (TP_UNID)
        let mut type_builder = StringBuilder::with_capacity(num_rows, num_rows * 4);
        for i in 0..num_rows {
            let tp = get_str_value(raw_batch, "TP_UNID", i).unwrap_or("00");
            type_builder.append_value(tp);
        }
        let type_col: ArrayRef = Arc::new(type_builder.finish());

        // 7. has_emergency_care (ATEND_PRONTO_SOCORRO / ATENDAMB)
        let mut urg_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let has_urg = get_bool_value(raw_batch, "ATEND_PRONTO_SOCORRO", i)
                .or_else(|| get_bool_value(raw_batch, "ATENDURG", i))
                .unwrap_or(false);
            urg_builder.append_value(has_urg);
        }
        let urg_col: ArrayRef = Arc::new(urg_builder.finish());

        // 8. total_surgical_beds (QTLEITOCIRURGICO)
        let mut surg_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let b = get_u16_value(raw_batch, "QTLEITOCIRURGICO", i).unwrap_or(0);
            surg_builder.append_value(b);
        }
        let surg_col: ArrayRef = Arc::new(surg_builder.finish());

        // 9. total_clinical_beds (QTLEITOCLINICO)
        let mut clin_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let b = get_u16_value(raw_batch, "QTLEITOCLINICO", i).unwrap_or(0);
            clin_builder.append_value(b);
        }
        let clin_col: ArrayRef = Arc::new(clin_builder.finish());

        // 10. total_icu_beds_sus (QTLEITOUTI_SUS)
        let mut uti_sus_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let b = get_u16_value(raw_batch, "QTLEITOUTI_SUS", i).unwrap_or(0);
            uti_sus_builder.append_value(b);
        }
        let uti_sus_col: ArrayRef = Arc::new(uti_sus_builder.finish());

        // 11. total_icu_beds_non_sus (QTLEITOUTI_NAOSUS)
        let mut uti_nonsus_builder = UInt16Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let b = get_u16_value(raw_batch, "QTLEITOUTI_NAOSUS", i).unwrap_or(0);
            uti_nonsus_builder.append_value(b);
        }
        let uti_nonsus_col: ArrayRef = Arc::new(uti_nonsus_builder.finish());

        // 12. competence_year_month (COMPETEN)
        let mut comp_builder = StringBuilder::with_capacity(num_rows, num_rows * 6);
        for i in 0..num_rows {
            let comp = get_str_value(raw_batch, "COMPETEN", i).unwrap_or("202401");
            comp_builder.append_value(comp);
        }
        let comp_col: ArrayRef = Arc::new(comp_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                cnes_id_col,
                name_col,
                mun_col,
                h3_col,
                mgmt_col,
                type_col,
                urg_col,
                surg_col,
                clin_col,
                uti_sus_col,
                uti_nonsus_col,
                comp_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for CnesDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.cnes",
            display_name: "CNES - Cadastro Nacional de Estabelecimentos de Saúde",
            maintaining_agency: "DATASUS / Ministério da Saúde",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::AssistanceInfrastructure,
            temporal_resolution: "Mensal",
            spatial_resolution: "Estabelecimento / Município (IBGE)",
            supported_years: 2005..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_health_facility_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let sub_type = params
            .extra_filters
            .get("type")
            .map(|s| s.as_str())
            .unwrap_or("ST"); // ST = Estabelecimentos, LT = Leitos
        let year_short = params.year % 100;
        let month = params.month.unwrap_or(1);
        let filename = format!(
            "{}{}{:02}{:02}.dbc",
            sub_type.to_uppercase(),
            uf.to_uppercase(),
            year_short,
            month
        );

        Ok(format!(
            "ftp://ftp.datasus.gov.br/dissemin/publicos/CNES/200508_/Dados/{sub_type}/{filename}"
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
