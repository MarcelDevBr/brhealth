// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Decodificador colunar de tabelas DBF diretamente para Apache Arrow `RecordBatch`.
//!
//! Constrói vetores de memória contígua alinhados a 64 bytes sem serializações intermediárias,
//! oferecendo suporte nativo para strings aparadas, inteiros, floats, datas e booleanos.

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, Float64Builder, Int64Builder, StringBuilder,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use chrono::NaiveDate;

use crate::decoders::dbf::header::{DbfFieldDescriptor, DbfFieldType, DbfHeader};
use crate::domain::ports::outbound::PortError;

enum ColumnBuilder {
    Utf8(StringBuilder),
    Int64(Int64Builder),
    Float64(Float64Builder),
    Date32(Date32Builder),
    Boolean(BooleanBuilder),
}

impl ColumnBuilder {
    fn new(desc: &DbfFieldDescriptor, capacity: usize) -> Self {
        match desc.field_type {
            DbfFieldType::Character | DbfFieldType::Other(_) => Self::Utf8(
                StringBuilder::with_capacity(capacity, capacity * desc.length),
            ),
            DbfFieldType::Numeric => {
                if desc.decimal_count == 0 {
                    Self::Int64(Int64Builder::with_capacity(capacity))
                } else {
                    Self::Float64(Float64Builder::with_capacity(capacity))
                }
            }
            DbfFieldType::Float => Self::Float64(Float64Builder::with_capacity(capacity)),
            DbfFieldType::Date => Self::Date32(Date32Builder::with_capacity(capacity)),
            DbfFieldType::Logical => Self::Boolean(BooleanBuilder::with_capacity(capacity)),
        }
    }

    fn append_value(&mut self, raw_bytes: &[u8]) {
        match self {
            Self::Utf8(builder) => {
                if let Ok(s) = std::str::from_utf8(raw_bytes) {
                    let trimmed = s.trim_end();
                    if trimmed.is_empty() {
                        builder.append_null();
                    } else {
                        builder.append_value(trimmed);
                    }
                } else {
                    let text = String::from_utf8_lossy(raw_bytes);
                    let trimmed = text.trim_end();
                    if trimmed.is_empty() {
                        builder.append_null();
                    } else {
                        builder.append_value(trimmed);
                    }
                }
            }
            Self::Int64(builder) => {
                let Ok(s) = std::str::from_utf8(raw_bytes) else {
                    builder.append_null();
                    return;
                };
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    builder.append_null();
                } else if let Ok(val) = trimmed.parse::<i64>() {
                    builder.append_value(val);
                } else {
                    builder.append_null();
                }
            }
            Self::Float64(builder) => {
                let Ok(s) = std::str::from_utf8(raw_bytes) else {
                    builder.append_null();
                    return;
                };
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    builder.append_null();
                } else if let Ok(val) = trimmed.parse::<f64>() {
                    builder.append_value(val);
                } else {
                    builder.append_null();
                }
            }
            Self::Date32(builder) => {
                let Ok(s) = std::str::from_utf8(raw_bytes) else {
                    builder.append_null();
                    return;
                };
                let trimmed = s.trim();
                let parsed_days = if trimmed.len() == 8 {
                    NaiveDate::parse_from_str(trimmed, "%Y%m%d")
                        .ok()
                        .and_then(|date| {
                            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?;
                            i32::try_from(date.signed_duration_since(epoch).num_days()).ok()
                        })
                } else {
                    None
                };

                if let Some(days) = parsed_days {
                    builder.append_value(days);
                } else {
                    builder.append_null();
                }
            }
            Self::Boolean(builder) => {
                let trimmed = raw_bytes.iter().find(|&&b| b != b' ');
                match trimmed {
                    Some(b'T' | b't' | b'Y' | b'y') => builder.append_value(true),
                    Some(b'F' | b'f' | b'N' | b'n') => builder.append_value(false),
                    _ => builder.append_null(),
                }
            }
        }
    }

    fn finish(self) -> ArrayRef {
        match self {
            Self::Utf8(mut b) => Arc::new(b.finish()),
            Self::Int64(mut b) => Arc::new(b.finish()),
            Self::Float64(mut b) => Arc::new(b.finish()),
            Self::Date32(mut b) => Arc::new(b.finish()),
            Self::Boolean(mut b) => Arc::new(b.finish()),
        }
    }
}

/// Decodificador de tabelas dBase/DBF para Apache Arrow.
#[derive(Debug, Default, Clone)]
pub struct DbfDecoder;

impl DbfDecoder {
    /// Cria uma nova instância de `DbfDecoder`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Deriva o esquema canônico Arrow (`Arc<Schema>`) a partir dos descritores DBF.
    #[must_use]
    pub fn build_arrow_schema(header: &DbfHeader) -> Arc<Schema> {
        let fields: Vec<Field> = header
            .fields
            .iter()
            .map(|f| {
                let data_type = match f.field_type {
                    DbfFieldType::Character | DbfFieldType::Other(_) => DataType::Utf8,
                    DbfFieldType::Numeric => {
                        if f.decimal_count == 0 {
                            DataType::Int64
                        } else {
                            DataType::Float64
                        }
                    }
                    DbfFieldType::Float => DataType::Float64,
                    DbfFieldType::Date => DataType::Date32,
                    DbfFieldType::Logical => DataType::Boolean,
                };
                Field::new(&f.name, data_type, true)
            })
            .collect();

        Arc::new(Schema::new(fields))
    }

    /// Converte um buffer com uma tabela DBF completa em um `RecordBatch` Apache Arrow.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::TabularDecodeError` caso o cabeçalho DBF seja corrompido,
    /// os dados estejam truncados ou ocorra falha na montagem do `RecordBatch`.
    pub fn decode_to_record_batch(&self, input: &[u8]) -> Result<RecordBatch, PortError> {
        let header = DbfHeader::parse(input)?;
        let schema = Self::build_arrow_schema(&header);

        let capacity = header.records_count;
        let mut builders: Vec<ColumnBuilder> = header
            .fields
            .iter()
            .map(|f| ColumnBuilder::new(f, capacity))
            .collect();

        for i in 0..header.records_count {
            let record_start = header.header_size + i * header.record_size;
            if record_start + header.record_size > input.len() {
                // Registros terminaram antes do contador declarado no cabeçalho
                break;
            }

            let record_data = &input[record_start..record_start + header.record_size];

            // Byte 0 indica status de exclusão: 0x20 (' ') = válido, 0x2A ('*') = excluído
            if record_data[0] == b'*' {
                // Registro marcado como deletado no DBF é ignorado
                continue;
            }

            for (col_idx, field) in header.fields.iter().enumerate() {
                let start = 1 + field.offset;
                let end = start + field.length;
                if end <= record_data.len() {
                    builders[col_idx].append_value(&record_data[start..end]);
                } else {
                    builders[col_idx].append_value(&[]);
                }
            }
        }

        let columns: Vec<ArrayRef> = builders.into_iter().map(ColumnBuilder::finish).collect();

        RecordBatch::try_new(schema, columns).map_err(|e| {
            PortError::TabularDecodeError(format!(
                "Falha na criação do RecordBatch Apache Arrow a partir do DBF: {e}"
            ))
        })
    }
}
