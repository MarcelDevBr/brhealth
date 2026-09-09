// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Decodificador canônico de árvores de Huffman para o formato PKWARE DCL / Blast.
//!
//! Implementa a reconstrução e navegação segura por árvores de Huffman com profundidade
//! máxima de 13 bits conforme a especificação do algoritmo original de Mark Adler.

use crate::decoders::blast::bit_reader::BitReader;
use crate::domain::ports::outbound::PortError;

/// Número máximo de bits de um código canônico no PKWARE DCL.
pub const MAX_BITS: usize = 13;

/// Estrutura contendo contagens e símbolos da árvore canônica de Huffman.
#[derive(Debug, Clone)]
pub struct HuffmanTree {
    count: [i32; MAX_BITS + 1],
    symbols: Vec<u16>,
}

impl HuffmanTree {
    /// Constrói uma nova árvore de Huffman a partir da especificação compactada de comprimentos.
    ///
    /// Cada byte na fatia `rep` codifica:
    /// - 4 bits mais significativos: `(count - 1)` de repetições (entre 1 e 16).
    /// - 4 bits menos significativos: comprimento em bits do código (entre 0 e 13).
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` se a tabela de códigos for inválida,
    /// não possuir códigos ou violar a desigualdade de Kraft-McMillan.
    pub fn from_lengths_repr(rep: &[u8]) -> Result<Self, PortError> {
        let mut lengths = Vec::new();
        for &val in rep {
            let count = ((val >> 4) + 1) as usize;
            let bit_len = val & 0x0F;
            if bit_len as usize > MAX_BITS {
                return Err(PortError::DecompressionError(format!(
                    "Comprimento de código Huffman inválido: {bit_len} (máximo permitido: {MAX_BITS})"
                )));
            }
            lengths.extend(std::iter::repeat_n(bit_len, count));
        }

        let mut count = [0i32; MAX_BITS + 1];
        for &l in &lengths {
            count[l as usize] += 1;
        }

        if count[0] as usize == lengths.len() {
            return Err(PortError::DecompressionError(
                "Árvore de Huffman inválida: nenhum código presente".to_string(),
            ));
        }

        // Validação da desigualdade de Kraft-McMillan
        let mut kraft_remaining = 1i32;
        for c in count.iter().skip(1) {
            kraft_remaining <<= 1;
            kraft_remaining -= *c;
            if kraft_remaining < 0 {
                return Err(PortError::DecompressionError(
                    "Árvore de Huffman sobre-subscrita (violação da desigualdade de Kraft)"
                        .to_string(),
                ));
            }
        }

        let mut offsets = [0usize; MAX_BITS + 1];
        for i in 1..MAX_BITS {
            offsets[i + 1] = offsets[i] + count[i] as usize;
        }

        let mut symbols = vec![0u16; lengths.len()];
        for (i, &l) in lengths.iter().enumerate() {
            if l != 0 {
                let off_idx = l as usize;
                let target_idx = offsets[off_idx];
                if target_idx < symbols.len() {
                    symbols[target_idx] = i as u16;
                    offsets[off_idx] += 1;
                }
            }
        }

        Ok(Self { count, symbols })
    }

    /// Decodifica o próximo símbolo a partir do fluxo de bits.
    ///
    /// De acordo com o algoritmo PKWARE DCL / blast, a leitura inverte o bit recebido (`bit ^ 1`).
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` se o código não puder ser resolvido
    /// ou se o fluxo binário terminar antes do término do símbolo.
    #[inline]
    pub fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, PortError> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0usize;

        for length in 1..=MAX_BITS {
            let bit = reader.read_bit()? as i32;
            code = (code << 1) | (bit ^ 1);
            let cnt = self.count[length];

            if code < first + cnt {
                let sym_idx = index + (code - first) as usize;
                return self.symbols.get(sym_idx).copied().ok_or_else(|| {
                    PortError::DecompressionError(
                        "Índice de símbolo Huffman decodificado fora dos limites".to_string(),
                    )
                });
            }

            index += cnt as usize;
            first = (first + cnt) << 1;
        }

        Err(PortError::DecompressionError(
            "Código Huffman não encontrado dentro da profundidade máxima permitida".to_string(),
        ))
    }
}
