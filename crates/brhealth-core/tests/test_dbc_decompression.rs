// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use brhealth_core::decoders::DbcDecompressor;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_decompress_real_datasus_dbc_fixture() {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push("tests/fixtures/doac2022.dbc");

    if !fixture_path.exists() {
        eprintln!(
            "Fixture não encontrada em: {:?}, pulando teste",
            fixture_path
        );
        return;
    }

    let compressed_bytes = fs::read(&fixture_path).expect("Falha ao ler fixture doac2022.dbc");
    assert!(!compressed_bytes.is_empty());

    let decompressor = DbcDecompressor::new().expect("Falha ao inicializar DbcDecompressor");
    let decompressed = decompressor
        .decompress_dbc(&compressed_bytes)
        .expect("Falha na descompressão do arquivo .dbc real");

    // Validações no cabeçalho DBF resultante
    assert!(
        decompressed.len() > 2_000_000,
        "Tamanho descomprimido esperado ~2.04 MB, obtido: {}",
        decompressed.len()
    );

    // Byte 0 do DBF deve ser 0x03 (dBase III sem memo)
    assert_eq!(decompressed[0], 0x03, "Versão de DBF inválida");

    // Bytes 4..8: número de registros (u32 little-endian)
    let records_count = u32::from_le_bytes([
        decompressed[4],
        decompressed[5],
        decompressed[6],
        decompressed[7],
    ]);
    assert_eq!(
        records_count, 4159,
        "Contagem de registros esperada de 4159 óbitos no Acre 2022"
    );

    // Bytes 8..10: tamanho do cabeçalho
    let header_size = u16::from_le_bytes([decompressed[8], decompressed[9]]) as usize;
    assert_eq!(header_size, 2817);

    // Byte terminador do cabeçalho (posição header_size - 1) deve ser 0x0D ('\r')
    assert_eq!(
        decompressed[header_size - 1],
        0x0D,
        "Terminador de cabeçalho DBF deve ser 0x0D"
    );

    // Bytes 10..12: tamanho de cada registro
    let record_len = u16::from_le_bytes([decompressed[10], decompressed[11]]) as usize;
    assert_eq!(record_len, 490);

    // O tamanho total deve ser exatamente header_size + records_count * record_len (+ opcional 1 byte 0x1A de EOF)
    let expected_data_len = header_size + (records_count as usize) * record_len;
    assert!(
        decompressed.len() >= expected_data_len,
        "Tamanho obtido ({}) menor que o esperado ({expected_data_len})",
        decompressed.len()
    );

    // Decodificação DBF para Apache Arrow RecordBatch
    let dbf_decoder = brhealth_core::decoders::dbf::DbfDecoder::new();
    let record_batch = dbf_decoder
        .decode_to_record_batch(&decompressed)
        .expect("Falha ao decodificar DBF para Apache Arrow RecordBatch");

    assert_eq!(
        record_batch.num_rows(),
        4159,
        "RecordBatch deve conter exatamente 4.159 linhas"
    );
    assert!(
        record_batch.num_columns() > 50,
        "Tabela SIM deve conter mais de 50 colunas epidemiológicas"
    );

    // Validação de presença de campos canônicos do SIM
    let schema = record_batch.schema();
    assert!(
        schema.field_with_name("DTOBITO").is_ok(),
        "Campo DTOBITO deve existir"
    );
    assert!(
        schema.field_with_name("CAUSABAS").is_ok(),
        "Campo CAUSABAS deve existir"
    );
    assert!(
        schema.field_with_name("CODMUNRES").is_ok(),
        "Campo CODMUNRES deve existir"
    );
}
