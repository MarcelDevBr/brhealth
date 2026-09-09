---
name: datasus-blast-decoder
description: >-
  Use this skill when implementing, debugging or optimizing the native Rust PKWARE DCL (.dbc)
  decompressor, DBF columnar decoder, or dealing with DATASUS legacy file formats.
---

# DATASUS Blast & DBF Decoder Skill

Esta skill orienta a engenharia de decodificação de formatos legados do Ministério da Saúde / DATASUS com máxima segurança de memória, ausência de binários externos em C e cobertura de testes de robustez.

## 1. Especificação do Formato .dbc (PKWARE DCL)
- Os arquivos `.dbc` disseminados pelo DATASUS são tabelas `.dbf` compactadas com a biblioteca **PKWARE Data Compression Library (DCL)**.
- **Modo**: Binário (tipo 0).
- **Tamanho do Dicionário**: Janela deslizante circular de 4096 bytes (parâmetro de dicionário = 6, $2^6 \times 64 = 4096$).
- **Codificação de Bits**: Codificação canônica de Huffman pré-definida para comprimentos e distâncias.

### Estrutura do Fluxo de Bits
1. Se bit = `0`:
   - Próximos 8 bits representam um byte literal direto.
   - Escreve no buffer de saída e atualiza a janela deslizante de 4096 bytes.
2. Se bit = `1`:
   - Decodifica o código de comprimento (árvore de Huffman de comprimentos) + bits extras da tabela `LENGTH_BASE` e `LENGTH_EXTRA_BITS`.
   - Decodifica o código de distância (árvore de Huffman de distâncias) + bits extras da tabela `DISTANCE_EXTRA_BITS`.
   - Executa a cópia circular do histórico a partir de `(win_pos + 4096 - distance) % 4096`.

## 2. Decodificação Colunar DBF para Apache Arrow
- Os arquivos DBF descomprimidos possuem cabeçalho com número de registros e descrição de campos (nome, tipo `C`, `N`, `D`, tamanho, decimais).
- A conversão colunar deve:
  - Alocar `arrow::array::Builder` apropriado para cada tipo de coluna.
  - Remover espaços à direita em strings (`trim_end`).
  - Converter inteiros e floats diretamente sem alocação intermediária.
  - Converter datas `YYYYMMDD` para `arrow::datatypes::Date32Type`.

## 3. Testes de Robustez, Fuzzing e Memory Safety
- Decodificadores binários devem ser submetidos a testes com:
  - Fluxos vazios (`input.is_empty()`).
  - Fluxos truncados prematuramente no meio de códigos Huffman.
  - Códigos de distância maiores que o histórico disponível.
  - Payloads aleatórios (*fuzz testing*).
- O decodificador **nunca deve entrar em loop infinito nem causar panic**, retornando sempre `Err(PortError::DecompressionError(...))`.
