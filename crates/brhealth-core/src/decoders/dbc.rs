// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Descompressor de arquivos de banco de dados comprimidos do DATASUS (.dbc).
//!
//! O formato `.dbc` consiste em uma tabela dBase III/IV (.dbf) cujo cabeçalho de metadados
//! é preservado intacto (com terminador 0x0D) e o corpo de registros tabulares é compactado
//! com o algoritmo PKWARE Data Compression Library (DCL / Blast).

use crate::decoders::blast::BlastDecompressor;
use crate::domain::ports::outbound::{DecompressorPort, PortError};

/// Descompressor de arquivos `.dbc` do DATASUS implementando a porta `DecompressorPort`.
#[derive(Debug, Default, Clone)]
pub struct DbcDecompressor {
    blast: BlastDecompressor,
}

impl DbcDecompressor {
    /// Cria uma nova instância do decodificador DBC.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` se a inicialização do descompressor Blast falhar.
    pub fn new() -> Result<Self, PortError> {
        Ok(Self {
            blast: BlastDecompressor::new()?,
        })
    }

    /// Descomprime um arquivo ou buffer `.dbc` em bytes de uma tabela `.dbf` canônica.
    ///
    /// # Estrutura do Contêiner DATASUS .dbc
    ///
    /// 1. `bytes[0..=7]`: Versão do dBase e metadados de registros.
    /// 2. `bytes[8..=9]`: Tamanho do cabeçalho DBF em formato Little-Endian (`header_size`).
    /// 3. `bytes[0..header_size - 1]`: Descritores de campos DBF.
    /// 4. O byte `header_size - 1` é finalizado com o terminador `0x0D` (`\r`).
    /// 5. `bytes[header_size..header_size + 4]`: Marcador de controle/tamanho do DATASUS (4 bytes pulados).
    /// 6. `bytes[header_size + 4..]`: Fluxo binário PKWARE DCL descomprimido com o algoritmo Blast.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` caso o arquivo seja truncado, possua tamanho de cabeçalho
    /// inválido ou ocorra falha na decodificação do fluxo DCL.
    pub fn decompress_dbc(&self, input: &[u8]) -> Result<Vec<u8>, PortError> {
        if input.len() < 14 {
            return Err(PortError::DecompressionError(format!(
                "Arquivo .dbc inválido ou truncado: possui apenas {} bytes (mínimo exigido: 14)",
                input.len()
            )));
        }

        let header_size = (input[8] as usize) | ((input[9] as usize) << 8);
        if header_size < 32 {
            return Err(PortError::DecompressionError(format!(
                "Tamanho do cabeçalho DBF inválido: {header_size} bytes (mínimo esperado: 32 bytes)"
            )));
        }

        if header_size + 4 > input.len() {
            return Err(PortError::DecompressionError(format!(
                "Inconsistência de cabeçalho DBC: header_size ({header_size}) + 4 excede o tamanho total ({})",
                input.len()
            )));
        }

        // Cabeçalho da tabela DBF reconstruído com o terminador canônico 0x0D
        let mut decompressed = Vec::with_capacity(header_size + (input.len() - header_size) * 4);
        decompressed.extend_from_slice(&input[..header_size - 1]);
        decompressed.push(0x0D);

        // Descompressão Blast do corpo tabular
        let compressed_body = &input[header_size + 4..];
        let decompressed_body = self.blast.decompress(compressed_body)?;
        decompressed.extend_from_slice(&decompressed_body);

        Ok(decompressed)
    }
}

impl DecompressorPort for DbcDecompressor {
    fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, PortError> {
        self.decompress_dbc(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_empty_input() {
        let decompressor = DbcDecompressor::new().unwrap();
        assert!(decompressor.decompress(&[]).is_err());
    }

    #[test]
    fn test_rejects_truncated_header() {
        let decompressor = DbcDecompressor::new().unwrap();
        let short_data = [3u8; 10];
        assert!(decompressor.decompress(&short_data).is_err());
    }
}
