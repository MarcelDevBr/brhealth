// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Leitor de fluxo de bits de alta performance para o descompressor Blast PKWARE DCL.
//!
//! Opera sobre fatias contíguas de memória (`&[u8]`) com leitura LSB-first (Least Significant Bit),
//! mantendo buffer interno de 64 bits para minimizar acessos à memória e garantir zero alocações.

use crate::domain::ports::outbound::PortError;

/// Leitor de bits LSB-first seguro e sem alocações.
#[derive(Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u64,
    bit_cnt: u32,
}

impl<'a> BitReader<'a> {
    /// Cria um novo `BitReader` a partir de uma fatia de bytes.
    #[inline]
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buf: 0,
            bit_cnt: 0,
        }
    }

    /// Retorna a posição atual do ponteiro de bytes de entrada.
    #[inline]
    #[must_use]
    pub fn byte_position(&self) -> usize {
        self.pos
    }

    /// Retorna quantos bytes ainda restam no buffer de entrada não consumido.
    #[inline]
    #[must_use]
    pub fn remaining_bytes(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Lê `need` bits (entre 0 e 32) do fluxo no formato LSB-first.
    ///
    /// # Erros
    ///
    /// Retorna `PortError::DecompressionError` se o fluxo de entrada terminar antes
    /// de satisfazer a quantidade de bits solicitada ou se `need > 32`.
    #[inline]
    pub fn read_bits(&mut self, need: u32) -> Result<u32, PortError> {
        if need == 0 {
            return Ok(0);
        }
        if need > 32 {
            return Err(PortError::DecompressionError(format!(
                "Leitura de bits excessiva solicitada: {need} bits (máximo suportado: 32)"
            )));
        }

        while self.bit_cnt < need {
            if self.pos >= self.data.len() {
                return Err(PortError::DecompressionError(format!(
                    "Fim prematuro do fluxo binário: necessários {need} bits, disponíveis apenas {}",
                    self.bit_cnt
                )));
            }
            let byte = self.data[self.pos] as u64;
            self.bit_buf |= byte << self.bit_cnt;
            self.bit_cnt += 8;
            self.pos += 1;
        }

        let mask = if need == 32 {
            u32::MAX
        } else {
            (1u32 << need) - 1
        };

        let result = (self.bit_buf as u32) & mask;
        self.bit_buf >>= need;
        self.bit_cnt -= need;

        Ok(result)
    }

    /// Lê 1 bit do fluxo binário.
    #[inline]
    pub fn read_bit(&mut self) -> Result<u8, PortError> {
        self.read_bits(1).map(|b| b as u8)
    }

    /// Lê 1 byte literal (8 bits).
    #[inline]
    pub fn read_byte(&mut self) -> Result<u8, PortError> {
        self.read_bits(8).map(|b| b as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_reader() {
        let mut reader = BitReader::new(&[]);
        assert_eq!(reader.read_bits(0).unwrap(), 0);
        assert!(reader.read_bits(1).is_err());
    }

    #[test]
    fn test_read_bits_lsb_first() {
        // 0b10110001 = 0xB1
        // LSB 4 bits: 0b0001 = 1
        // MSB 4 bits: 0b1011 = 11
        let data = [0xB1];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(4).unwrap(), 1);
        assert_eq!(reader.read_bits(4).unwrap(), 11);
        assert!(reader.read_bits(1).is_err());
    }

    #[test]
    fn test_cross_byte_read() {
        // Byte 0: 0xFF (11111111), Byte 1: 0x00 (00000000)
        // 12 bits read: 8 bits of 1s followed by 4 bits of 0s = 0x0FF = 255
        let data = [0xFF, 0x00];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(12).unwrap(), 0x0FF);
        assert_eq!(reader.read_bits(4).unwrap(), 0);
    }
}
