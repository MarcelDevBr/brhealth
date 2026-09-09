// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Implementação nativa em Rust do algoritmo de descompressão Blast PKWARE DCL.

pub mod bit_reader;
pub mod decompressor;
pub mod huffman;

pub use bit_reader::BitReader;
pub use decompressor::BlastDecompressor;
pub use huffman::HuffmanTree;
