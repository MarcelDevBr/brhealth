// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Parser do cabeçalho e descritores de campos de tabelas dBase III / IV (.dbf).

use crate::domain::ports::outbound::PortError;

/// Tipo de dado de um campo de tabela DBF conforme especificação xBase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbfFieldType {
    /// Caracteres alfanuméricos preenchidos com espaços à direita ('C')
    Character,
    /// Numérico em formato ASCII ('N')
    Numeric,
    /// Ponto flutuante IEEE ou ASCII ('F')
    Float,
    /// Data no formato YYYYMMDD ('D')
    Date,
    /// Lógico booleano T/F/Y/N ('L')
    Logical,
    /// Outros tipos não analíticos ('M' para Memo, etc.)
    Other(u8),
}

impl From<u8> for DbfFieldType {
    fn from(b: u8) -> Self {
        match b {
            b'C' => Self::Character,
            b'N' => Self::Numeric,
            b'F' => Self::Float,
            b'D' => Self::Date,
            b'L' => Self::Logical,
            other => Self::Other(other),
        }
    }
}

/// Descritor de um campo na tabela DBF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbfFieldDescriptor {
    /// Nome do campo (até 10 caracteres alfanuméricos ASCII)
    pub name: String,
    /// Tipo de dado do campo
    pub field_type: DbfFieldType,
    /// Comprimento em bytes do campo no registro
    pub length: usize,
    /// Casas decimais (para tipos numéricos)
    pub decimal_count: u8,
    /// Deslocamento relativo ao início dos dados do registro (após o byte de deleção)
    pub offset: usize,
}

/// Metadados e descritores extraídos do cabeçalho de uma tabela DBF.
#[derive(Debug, Clone)]
pub struct DbfHeader {
    /// Versão do formato dBase (ex: 0x03)
    pub version: u8,
    /// Quantidade total de registros na tabela
    pub records_count: usize,
    /// Tamanho total do cabeçalho em bytes
    pub header_size: usize,
    /// Tamanho total de cada registro em bytes (incluindo o byte de status de deleção)
    pub record_size: usize,
    /// Lista dos descritores de campos
    pub fields: Vec<DbfFieldDescriptor>,
}

impl DbfHeader {
    /// Faz o parsing do cabeçalho DBF a partir de um buffer de bytes.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::TabularDecodeError` caso o cabeçalho seja menor que 32 bytes,
    /// contenha tamanhos inconsistentes ou não encontre o terminador `0x0D`.
    pub fn parse(input: &[u8]) -> Result<Self, PortError> {
        if input.len() < 32 {
            return Err(PortError::TabularDecodeError(format!(
                "Cabeçalho DBF truncado: tamanho {} bytes (mínimo exigido: 32)",
                input.len()
            )));
        }

        let version = input[0];
        let records_count = u32::from_le_bytes([input[4], input[5], input[6], input[7]]) as usize;
        let header_size = u16::from_le_bytes([input[8], input[9]]) as usize;
        let record_size = u16::from_le_bytes([input[10], input[11]]) as usize;

        if header_size < 32 || header_size > input.len() {
            return Err(PortError::TabularDecodeError(format!(
                "Tamanho do cabeçalho DBF inválido: {header_size} (dados disponíveis: {})",
                input.len()
            )));
        }

        if record_size == 0 {
            return Err(PortError::TabularDecodeError(
                "Tamanho do registro DBF não pode ser zero".to_string(),
            ));
        }

        let mut fields = Vec::new();
        let mut field_pos = 32;
        let mut running_offset = 0;

        while field_pos + 32 <= header_size {
            // Terminador do array de descritores de campo é 0x0D
            if input[field_pos] == 0x0D {
                break;
            }

            let field_slice = &input[field_pos..field_pos + 32];

            // O nome do campo ocupa os primeiros 11 bytes, terminado em null ou espaço
            let name_bytes = &field_slice[0..11];
            let name_end = name_bytes
                .iter()
                .position(|&b| b == 0 || b == b' ')
                .unwrap_or(11);
            let name = String::from_utf8_lossy(&name_bytes[..name_end])
                .trim()
                .to_string();

            let field_type = DbfFieldType::from(field_slice[11]);
            let length = field_slice[16] as usize;
            let decimal_count = field_slice[17];

            fields.push(DbfFieldDescriptor {
                name,
                field_type,
                length,
                decimal_count,
                offset: running_offset,
            });

            running_offset += length;
            field_pos += 32;
        }

        Ok(Self {
            version,
            records_count,
            header_size,
            record_size,
            fields,
        })
    }
}
