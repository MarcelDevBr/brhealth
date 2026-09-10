// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Motor de descompressão de alta performance PKWARE DCL (Blast) 100% nativo em Rust.
//!
//! Implementa o algoritmo de descompressão por janela circular deslizante de 4096 bytes
//! sem alocações dinâmicas no loop quente e com tolerância zero a falhas ou acessos inválidos.

use crate::decoders::blast::bit_reader::BitReader;
use crate::decoders::blast::huffman::HuffmanTree;
use crate::domain::ports::outbound::PortError;

/// Tamanho máximo da janela deslizante circular em bytes (4 KiB).
pub const MAX_WINDOW_SIZE: usize = 4096;

/// Código de comprimento sentinela que demarca o fim do bloco/fluxo comprimido.
pub const END_OF_STREAM_LENGTH: usize = 519;

// Tabelas canônicas canônicas da especificação PKWARE DCL (Mark Adler - blast.c)
const LITLEN: [u8; 98] = [
    11, 124, 8, 7, 28, 7, 188, 13, 76, 4, 10, 8, 12, 10, 12, 10, 8, 23, 8, 9, 7, 6, 7, 8, 7, 6, 55,
    8, 23, 24, 12, 11, 7, 9, 11, 12, 6, 7, 22, 5, 7, 24, 6, 11, 9, 6, 7, 22, 7, 11, 38, 7, 9, 8,
    25, 11, 8, 11, 9, 12, 8, 12, 5, 38, 5, 38, 5, 11, 7, 5, 6, 21, 6, 10, 53, 8, 7, 24, 10, 27, 44,
    253, 253, 253, 252, 252, 252, 13, 12, 45, 12, 45, 12, 61, 12, 45, 44, 173,
];

const LENLEN: [u8; 6] = [2, 35, 36, 53, 38, 23];

const DISTLEN: [u8; 7] = [2, 20, 53, 230, 247, 151, 248];

const BASE: [u16; 16] = [3, 2, 4, 5, 6, 7, 8, 9, 10, 12, 16, 24, 40, 72, 136, 264];

const EXTRA: [u32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];

/// Descompressor Blast nativo e thread-safe com árvores de Huffman pré-construídas.
#[derive(Debug, Clone)]
pub struct BlastDecompressor {
    litcode: HuffmanTree,
    lencode: HuffmanTree,
    distcode: HuffmanTree,
}

impl Default for BlastDecompressor {
    fn default() -> Self {
        match Self::new() {
            Ok(decompressor) => decompressor,
            Err(_) => Self {
                litcode: HuffmanTree::empty(),
                lencode: HuffmanTree::empty(),
                distcode: HuffmanTree::empty(),
            },
        }
    }
}

impl BlastDecompressor {
    /// Inicializa uma nova instância do descompressor compilando as árvores canônicas de Huffman.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` se houver anomalia na inicialização das tabelas.
    pub fn new() -> Result<Self, PortError> {
        let litcode = HuffmanTree::from_lengths_repr(&LITLEN)?;
        let lencode = HuffmanTree::from_lengths_repr(&LENLEN)?;
        let distcode = HuffmanTree::from_lengths_repr(&DISTLEN)?;

        Ok(Self {
            litcode,
            lencode,
            distcode,
        })
    }

    /// Descomprime um fluxo binário no padrão PKWARE DCL.
    ///
    /// # Formulação Matemática e Algorítmica
    ///
    /// Seja $W$ o histórico circular de tamanho $N = 4096$ bytes, e $p \in \mathbb{N}$ a posição
    /// monotônica do ponteiro de escrita:
    ///
    /// $$\text{pos}_{\text{origem}} = (p - \text{dist}) \pmod N$$
    ///
    /// Onde $\text{dist}$ é derivado de:
    ///
    /// $$\text{dist} = (\text{sym}_{\text{dist}} \ll \text{bits}_{\text{dist}}) + \text{extra}_{\text{dist}} + 1$$
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` caso o cabeçalho seja inválido, ocorra término
    /// inesperado dos bits, corrupção da árvore de Huffman ou referência a histórico anterior ao início.
    pub fn decompress(&self, compressed_data: &[u8]) -> Result<Vec<u8>, PortError> {
        if compressed_data.len() < 2 {
            return Err(PortError::DecompressionError(
                "Fluxo PKWARE DCL vazio ou truncado (menos de 2 bytes de cabeçalho)".to_string(),
            ));
        }

        let mut reader = BitReader::new(compressed_data);

        // Cabeçalho: 1 byte de tipo literal (0 = binary, 1 = ascii) e 1 byte de tamanho de dicionário (4, 5 ou 6)
        let lit_flag = reader.read_bits(8)?;
        if lit_flag > 1 {
            return Err(PortError::DecompressionError(format!(
                "Flag de literais inválida no cabeçalho DCL: {lit_flag} (esperado 0 ou 1)"
            )));
        }

        let dict_size_flag = reader.read_bits(8)?;
        if !(4..=6).contains(&dict_size_flag) {
            return Err(PortError::DecompressionError(format!(
                "Tamanho de dicionário inválido no cabeçalho DCL: {dict_size_flag} (esperado entre 4 e 6)"
            )));
        }

        let mut window = [0u8; MAX_WINDOW_SIZE];
        let mut next_pos: usize = 0;
        let mut first_window = true;
        let mut output = Vec::with_capacity(compressed_data.len().saturating_mul(3));

        loop {
            let is_match = reader.read_bit()?;
            if is_match == 1 {
                // Sequência comprimida (match em histórico)
                let len_sym = self.lencode.decode(&mut reader)? as usize;
                let extra_bits = *EXTRA.get(len_sym).ok_or_else(|| {
                    PortError::DecompressionError("Símbolo de comprimento fora do limite".into())
                })?;

                let length = (BASE[len_sym] as usize) + (reader.read_bits(extra_bits)? as usize);
                if length == END_OF_STREAM_LENGTH {
                    // Sentinela de término de fluxo atingida
                    break;
                }

                let dist_bits = if length == 2 { 2 } else { dict_size_flag };
                let dist_sym = self.distcode.decode(&mut reader)? as usize;
                let dist_extra = reader.read_bits(dist_bits)? as usize;
                let dist = (dist_sym << dist_bits) + dist_extra + 1;

                if first_window && dist > next_pos {
                    return Err(PortError::DecompressionError(format!(
                        "Distância de repetição ({dist}) ultrapassa o início do histórico gravado ({next_pos})"
                    )));
                }

                output.reserve(length);
                let mask = MAX_WINDOW_SIZE - 1;
                for _ in 0..length {
                    let from_pos = next_pos.wrapping_sub(dist) & mask;
                    let byte = window[from_pos];
                    window[next_pos & mask] = byte;
                    output.push(byte);
                    next_pos += 1;
                }
                if first_window && next_pos >= MAX_WINDOW_SIZE {
                    first_window = false;
                }
            } else {
                // Byte literal
                let byte = if lit_flag == 1 {
                    self.litcode.decode(&mut reader)? as u8
                } else {
                    reader.read_byte()?
                };

                let mask = MAX_WINDOW_SIZE - 1;
                window[next_pos & mask] = byte;
                output.push(byte);
                next_pos += 1;
                if first_window && next_pos >= MAX_WINDOW_SIZE {
                    first_window = false;
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_rejects_empty_data() {
        let decompressor = BlastDecompressor::default();
        assert!(decompressor.decompress(&[]).is_err());
    }

    #[test]
    fn test_rejects_invalid_lit_flag() {
        let decompressor = BlastDecompressor::default();
        // lit_flag = 2 (invalid)
        let data = [2u8, 6u8];
        assert!(decompressor.decompress(&data).is_err());
    }

    #[test]
    fn test_rejects_invalid_dict_size() {
        let decompressor = BlastDecompressor::default();
        // lit_flag = 0, dict_size = 3 (invalid, must be 4..=6)
        let data = [0u8, 3u8];
        assert!(decompressor.decompress(&data).is_err());

        // lit_flag = 0, dict_size = 7 (invalid)
        let data2 = [0u8, 7u8];
        assert!(decompressor.decompress(&data2).is_err());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_test_blast_never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let decompressor = BlastDecompressor::default();
            // O descompressor deve sempre retornar Ok ou Err, sem jamais causar panic
            let _ = decompressor.decompress(&data);
        }

        #[test]
        fn prop_test_dbc_never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let dbc = crate::decoders::DbcDecompressor::default();
            let _ = dbc.decompress_dbc(&data);
        }
    }
}
