// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Adaptador SPI do Cadastro Único para Programas Sociais (CadÚnico - MDS).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, StringBuilder, UInt8Builder, UInt64Builder};
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
use crate::sources::datasus::helpers::{get_bool_value, get_str_value, get_u8_value};

/// Adaptador SPI para o Cadastro Único (Vulnerabilidade Social e Pobreza) do MDS.
#[derive(Debug, Default, Clone)]
pub struct CadUnicoDataSource;

impl CadUnicoDataSource {
    /// Cria uma nova instância de `CadUnicoDataSource`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Harmoniza o lote de registros de famílias para o schema canônico de vulnerabilidade social.
    pub fn harmonize_batch(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let num_rows = raw_batch.num_rows();
        let target_schema = CanonicalSchemas::canonical_social_vulnerability_schema();

        // 1. family_id (CD_FAMILIA ou COD_FAMILIAR_FAM)
        let mut id_builder = StringBuilder::with_capacity(num_rows, num_rows * 12);
        for i in 0..num_rows {
            let id = get_str_value(raw_batch, "CD_FAMILIA", i)
                .or_else(|| get_str_value(raw_batch, "COD_FAMILIAR_FAM", i))
                .unwrap_or("");
            if id.is_empty() {
                id_builder.append_value(format!("FAM_{i}"));
            } else {
                id_builder.append_value(id);
            }
        }
        let id_col: ArrayRef = Arc::new(id_builder.finish());

        // 2. municipality_code (CD_IBGE ou CD_MUNICIPIO)
        let mut mun_builder = StringBuilder::with_capacity(num_rows, num_rows * 7);
        for i in 0..num_rows {
            let resolved = get_str_value(raw_batch, "CD_IBGE", i)
                .or_else(|| get_str_value(raw_batch, "CD_MUNICIPIO", i))
                .and_then(|m| harmonize_ibge_code(m).ok())
                .unwrap_or_else(|| "0000000".to_string());
            mun_builder.append_value(resolved);
        }
        let mun_col: ArrayRef = Arc::new(mun_builder.finish());

        // 3. h3_index_res8
        let mut h3_builder = UInt64Builder::with_capacity(num_rows);
        for _ in 0..num_rows {
            h3_builder.append_null();
        }
        let h3_col: ArrayRef = Arc::new(h3_builder.finish());

        // 4. is_extreme_poverty (MARC_PBF ou FX_RFPC)
        let mut pov_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let is_pov = get_bool_value(raw_batch, "MARC_PBF", i).unwrap_or(true);
            pov_builder.append_value(is_pov);
        }
        let pov_col: ArrayRef = Arc::new(pov_builder.finish());

        // 5. receives_income_transfer (MARC_BENEFICIO)
        let mut ben_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let ben = get_bool_value(raw_batch, "MARC_BENEFICIO", i).unwrap_or(true);
            ben_builder.append_value(ben);
        }
        let ben_col: ArrayRef = Arc::new(ben_builder.finish());

        // 6. number_of_family_members (QT_PESSOAS_FAM)
        let mut members_builder = UInt8Builder::with_capacity(num_rows);
        for i in 0..num_rows {
            let m = get_u8_value(raw_batch, "QT_PESSOAS_FAM", i).unwrap_or(3);
            members_builder.append_value(m);
        }
        let members_col: ArrayRef = Arc::new(members_builder.finish());

        // 7. has_piped_water (ST_AGUA_CANALIZADA)
        let mut water_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let w = get_bool_value(raw_batch, "ST_AGUA_CANALIZADA", i).unwrap_or(true);
            water_builder.append_value(w);
        }
        let water_col: ArrayRef = Arc::new(water_builder.finish());

        // 8. has_sewage_network (ST_REDE_ESGOTO)
        let mut sewage_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let s = get_bool_value(raw_batch, "ST_REDE_ESGOTO", i).unwrap_or(false);
            sewage_builder.append_value(s);
        }
        let sewage_col: ArrayRef = Arc::new(sewage_builder.finish());

        // 9. has_electricity (ST_ENERGIA_ELETRICA)
        let mut elec_builder = BooleanBuilder::with_capacity(num_rows);
        for i in 0..num_rows {
            let e = get_bool_value(raw_batch, "ST_ENERGIA_ELETRICA", i).unwrap_or(true);
            elec_builder.append_value(e);
        }
        let elec_col: ArrayRef = Arc::new(elec_builder.finish());

        RecordBatch::try_new(
            target_schema,
            vec![
                id_col,
                mun_col,
                h3_col,
                pov_col,
                ben_col,
                members_col,
                water_col,
                sewage_col,
                elec_col,
            ],
        )
        .map_err(|e| PortError::TransformationError(e.to_string()))
    }
}

#[async_trait]
impl HealthDataSourceSPI for CadUnicoDataSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "mds.cadunico",
            display_name: "CadÚnico - Cadastro Único para Programas Sociais",
            maintaining_agency: "Ministério do Desenvolvimento e Assistência Social (MDS)",
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            category: SourceCategory::SocioDemographic,
            temporal_resolution: "Mensal",
            spatial_resolution: "Município / Família (IBGE)",
            supported_years: 2003..=2026,
            requires_authentication: false,
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        CanonicalSchemas::canonical_social_vulnerability_schema()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<String, PortError> {
        let uf = params.jurisdiction_code.as_deref().unwrap_or("BR");
        let year = params.year;
        let filename = format!("cadunico_{}_{}.parquet", uf.to_lowercase(), year);

        Ok(format!(
            "https://dados.gov.br/dados/conjuntos-dados/cadunico/{filename}"
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
