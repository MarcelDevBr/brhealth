// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Motor Declarativo de Mapeamento Colunar (*Template Method / Blueprint Mapper*).
//!
//! Padroniza e desduplica a harmonização de microdados do DATASUS (SIH, SIM, SINASC, etc.)
//! para os schemas canônicos Apache Arrow alinhados a 64 bytes, garantindo segurança
//! de tipos, tratamento de nulos determinístico e eliminação de boilerplate repetitivo.

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanArray};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;

use super::helpers::{
    build_computed_bool_col, build_constant_str_col, build_date32_col, build_date32_opt_col,
    build_f64_col, build_harmonized_ibge_col, build_null_col, build_race_col, build_record_id_col,
    build_sex_col, build_sim_age_col, build_str_col, build_str_opt_col, build_u8_opt_col,
    build_u16_col, build_u16_opt_col, build_u32_col,
};
use crate::domain::ports::outbound::PortError;

/// Especificação declarativa de extração e conversão de uma coluna.
#[derive(Debug, Clone)]
pub enum ColumnExtractorSpec {
    /// Identificador canônico com prefixo padronizado (`record_id`).
    RecordId {
        source_col: &'static str,
        prefix: &'static str,
    },
    /// Código de município harmonizado para 7 dígitos canônicos com DV do IBGE.
    HarmonizedIbge { source_col: &'static str },
    /// Data em formato `Date32` com fallback padrão.
    Date32 {
        source_col: &'static str,
        default_val: i32,
    },
    /// Data em formato `Date32` opcional (nulo se ausente).
    Date32Opt { source_col: &'static str },
    /// String obrigatória com valor padrão caso nulo.
    Str {
        source_col: &'static str,
        default_val: &'static str,
    },
    /// String opcional (nulo se ausente ou vazia).
    StrOpt {
        source_col: &'static str,
        avg_len: usize,
    },
    /// Sexo biológico canônico ("M", "F", "U").
    Sex { source_col: &'static str },
    /// Raça/Etnia conforme nomenclatura padronizada IBGE/DATASUS.
    Race { source_col: &'static str },
    /// Idade codificada do SIM calculada em anos inteiros.
    SimAge { source_col: &'static str },
    /// Inteiro de 8 bits opcional.
    U8Opt { source_col: &'static str },
    /// Inteiro de 16 bits com valor padrão.
    U16 {
        source_col: &'static str,
        default_val: u16,
    },
    /// Inteiro de 16 bits opcional.
    U16Opt { source_col: &'static str },
    /// Inteiro de 32 bits com valor padrão.
    U32 {
        source_col: &'static str,
        default_val: u32,
    },
    /// Ponto flutuante `f64` com valor padrão.
    F64 {
        source_col: &'static str,
        default_val: f64,
    },
    /// Coluna com valor constante de string para todas as linhas.
    ConstantStr { value: &'static str },
    /// Coluna com valor booleano constante para todas as linhas.
    ConstantBool { value: bool },
    /// Booleano calculado via comparação de string (ex: "1" = true).
    ComputedBool {
        source_col: &'static str,
        true_value: &'static str,
    },
    /// Coluna inteiramente preenchida com nulos para o tipo Arrow especificado.
    Null(DataType),
}

/// Harmonizador declarativo de lotes colunares DATASUS (*Blueprint*).
#[derive(Debug, Clone)]
pub struct DatasusBatchHarmonizer {
    target_schema: Arc<Schema>,
    extractors: Vec<ColumnExtractorSpec>,
}

impl DatasusBatchHarmonizer {
    /// Cria um novo harmonizador apontando para o schema canônico de destino.
    #[must_use]
    pub fn new(target_schema: Arc<Schema>) -> Self {
        Self {
            target_schema,
            extractors: Vec::new(),
        }
    }

    /// Adiciona uma especificação de extrator de coluna ao blueprint.
    #[must_use]
    pub fn with_column(mut self, spec: ColumnExtractorSpec) -> Self {
        self.extractors.push(spec);
        self
    }

    /// Adiciona extrator de `record_id`.
    #[must_use]
    pub fn with_record_id(self, source_col: &'static str, prefix: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::RecordId { source_col, prefix })
    }

    /// Adiciona extrator de município harmonizado IBGE.
    #[must_use]
    pub fn with_harmonized_ibge(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::HarmonizedIbge { source_col })
    }

    /// Adiciona extrator de `Date32` com fallback padrão.
    #[must_use]
    pub fn with_date32(self, source_col: &'static str, default_val: i32) -> Self {
        self.with_column(ColumnExtractorSpec::Date32 {
            source_col,
            default_val,
        })
    }

    /// Adiciona extrator de `Date32` opcional.
    #[must_use]
    pub fn with_date32_opt(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::Date32Opt { source_col })
    }

    /// Adiciona extrator de string com fallback padrão.
    #[must_use]
    pub fn with_str(self, source_col: &'static str, default_val: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::Str {
            source_col,
            default_val,
        })
    }

    /// Adiciona extrator de string opcional.
    #[must_use]
    pub fn with_str_opt(self, source_col: &'static str, avg_len: usize) -> Self {
        self.with_column(ColumnExtractorSpec::StrOpt {
            source_col,
            avg_len,
        })
    }

    /// Adiciona extrator de sexo canônico.
    #[must_use]
    pub fn with_sex(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::Sex { source_col })
    }

    /// Adiciona extrator de raça/etnia canônica.
    #[must_use]
    pub fn with_race(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::Race { source_col })
    }

    /// Adiciona extrator de idade calculada do SIM.
    #[must_use]
    pub fn with_sim_age(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::SimAge { source_col })
    }

    /// Adiciona extrator de `u8` opcional.
    #[must_use]
    pub fn with_u8_opt(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::U8Opt { source_col })
    }

    /// Adiciona extrator de `u16` com fallback padrão.
    #[must_use]
    pub fn with_u16(self, source_col: &'static str, default_val: u16) -> Self {
        self.with_column(ColumnExtractorSpec::U16 {
            source_col,
            default_val,
        })
    }

    /// Adiciona extrator de `u16` opcional.
    #[must_use]
    pub fn with_u16_opt(self, source_col: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::U16Opt { source_col })
    }

    /// Adiciona extrator de `u32` com fallback padrão.
    #[must_use]
    pub fn with_u32(self, source_col: &'static str, default_val: u32) -> Self {
        self.with_column(ColumnExtractorSpec::U32 {
            source_col,
            default_val,
        })
    }

    /// Adiciona extrator de `f64` com fallback padrão.
    #[must_use]
    pub fn with_f64(self, source_col: &'static str, default_val: f64) -> Self {
        self.with_column(ColumnExtractorSpec::F64 {
            source_col,
            default_val,
        })
    }

    /// Adiciona coluna de string constante.
    #[must_use]
    pub fn with_constant_str(self, value: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::ConstantStr { value })
    }

    /// Adiciona coluna booleana constante.
    #[must_use]
    pub fn with_constant_bool(self, value: bool) -> Self {
        self.with_column(ColumnExtractorSpec::ConstantBool { value })
    }

    /// Adiciona coluna booleana computada via igualdade de string.
    #[must_use]
    pub fn with_computed_bool(self, source_col: &'static str, true_value: &'static str) -> Self {
        self.with_column(ColumnExtractorSpec::ComputedBool {
            source_col,
            true_value,
        })
    }

    /// Adiciona coluna inteiramente nula para o tipo Arrow especificado.
    #[must_use]
    pub fn with_null(self, data_type: DataType) -> Self {
        self.with_column(ColumnExtractorSpec::Null(data_type))
    }

    /// Harmoniza o lote bruto do DATASUS aplicando as regras declaradas no blueprint.
    pub fn harmonize(&self, raw_batch: &RecordBatch) -> Result<RecordBatch, PortError> {
        let expected_fields = self.target_schema.fields().len();
        if self.extractors.len() != expected_fields {
            return Err(PortError::SchemaMismatch(format!(
                "Quantidade de extratores declarados ({}) diverge dos campos no schema alvo ({})",
                self.extractors.len(),
                expected_fields
            )));
        }

        let num_rows = raw_batch.num_rows();
        let mut columns: Vec<ArrayRef> = Vec::with_capacity(self.extractors.len());

        for spec in &self.extractors {
            let col: ArrayRef = match spec {
                ColumnExtractorSpec::RecordId { source_col, prefix } => {
                    build_record_id_col(raw_batch, source_col, prefix, num_rows)
                }
                ColumnExtractorSpec::HarmonizedIbge { source_col } => {
                    build_harmonized_ibge_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::Date32 {
                    source_col,
                    default_val,
                } => build_date32_col(raw_batch, source_col, *default_val, num_rows),
                ColumnExtractorSpec::Date32Opt { source_col } => {
                    build_date32_opt_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::Str {
                    source_col,
                    default_val,
                } => build_str_col(raw_batch, source_col, default_val, num_rows),
                ColumnExtractorSpec::StrOpt {
                    source_col,
                    avg_len,
                } => build_str_opt_col(raw_batch, source_col, *avg_len, num_rows),
                ColumnExtractorSpec::Sex { source_col } => {
                    build_sex_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::Race { source_col } => {
                    build_race_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::SimAge { source_col } => {
                    build_sim_age_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::U8Opt { source_col } => {
                    build_u8_opt_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::U16 {
                    source_col,
                    default_val,
                } => build_u16_col(raw_batch, source_col, *default_val, num_rows),
                ColumnExtractorSpec::U16Opt { source_col } => {
                    build_u16_opt_col(raw_batch, source_col, num_rows)
                }
                ColumnExtractorSpec::U32 {
                    source_col,
                    default_val,
                } => build_u32_col(raw_batch, source_col, *default_val, num_rows),
                ColumnExtractorSpec::F64 {
                    source_col,
                    default_val,
                } => build_f64_col(raw_batch, source_col, *default_val, num_rows),
                ColumnExtractorSpec::ConstantStr { value } => {
                    build_constant_str_col(value, num_rows)
                }
                ColumnExtractorSpec::ConstantBool { value } => {
                    Arc::new(BooleanArray::from(vec![*value; num_rows]))
                }
                ColumnExtractorSpec::ComputedBool {
                    source_col,
                    true_value,
                } => build_computed_bool_col(raw_batch, source_col, true_value, num_rows),
                ColumnExtractorSpec::Null(dt) => build_null_col(dt, num_rows),
            };
            columns.push(col);
        }

        RecordBatch::try_new(self.target_schema.clone(), columns)
            .map_err(|e| PortError::SchemaMismatch(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::StringArray;
    use arrow::datatypes::Field;

    #[test]
    fn test_datasus_batch_harmonizer_declarative() {
        let raw_schema = Arc::new(Schema::new(vec![
            Field::new("N_AIH", DataType::Utf8, false),
            Field::new("MUNIC_RES", DataType::Utf8, false),
            Field::new("DIAG_PRINC", DataType::Utf8, false),
            Field::new("MORTE", DataType::Utf8, false),
        ]));

        let raw_batch = RecordBatch::try_new(
            raw_schema,
            vec![
                Arc::new(StringArray::from(vec!["1234567890", "9876543210"])),
                Arc::new(StringArray::from(vec!["355030", "330455"])),
                Arc::new(StringArray::from(vec!["J18", "I10"])),
                Arc::new(StringArray::from(vec!["1", "0"])),
            ],
        )
        .expect("batch creation failed");

        let target_schema = Arc::new(Schema::new(vec![
            Field::new("record_id", DataType::Utf8, false),
            Field::new("municipality", DataType::Utf8, false),
            Field::new("diagnosis", DataType::Utf8, false),
            Field::new("died", DataType::Boolean, false),
            Field::new("is_csap", DataType::Boolean, false),
        ]));

        let harmonizer = DatasusBatchHarmonizer::new(target_schema)
            .with_record_id("N_AIH", "AIH")
            .with_harmonized_ibge("MUNIC_RES")
            .with_str("DIAG_PRINC", "Z00")
            .with_computed_bool("MORTE", "1")
            .with_constant_bool(false);

        let harmonized = harmonizer
            .harmonize(&raw_batch)
            .expect("harmonization failed");

        assert_eq!(harmonized.num_rows(), 2);
        assert_eq!(harmonized.num_columns(), 5);

        let ibge_col = harmonized
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("downcast failed");
        assert_eq!(ibge_col.value(0), "3550308"); // 355030 harmonizado para 7 dígitos
        assert_eq!(ibge_col.value(1), "3304557");

        let died_col = harmonized
            .column(3)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .expect("downcast failed");
        assert!(died_col.value(0));
        assert!(!died_col.value(1));
    }
}
