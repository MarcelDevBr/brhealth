# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Decodificação Nativa DATASUS e Interoperabilidade Zero-Copy:
1. Descompressão nativa em Rust do algoritmo Blast PKWARE DCL (arquivos .dbc).
2. Decodificação colunar de tabelas dBase III/IV (.dbf) em memória contígua Apache Arrow.
3. Travessia Zero-Copy para Polars, Pandas e PyArrow via Arrow C Data Interface.
4. Inspeção analítica de microdados reais de mortalidade (SIM-SUS).
5. Descompressão direta de .dbc para fluxo de bytes / arquivo .dbf em disco.
"""

import warnings
from pathlib import Path
import brhealth

# Suprimir avisos transitórios de migração de bibliotecas
warnings.filterwarnings("ignore", category=FutureWarning)


def main() -> None:
    print("=" * 70)
    print("  BRHealth: Decodificação Nativa DATASUS e Zero-Copy (Python API)")
    print("=" * 70)

    # 1. Localizar arquivo de fixture real DATASUS (.dbc) incluído no projeto
    repo_root = Path(__file__).resolve().parent.parent
    fixture_path = repo_root / "crates" / "brhealth-core" / "tests" / "fixtures" / "doac2022.dbc"

    if not fixture_path.exists():
        print(f"[-] Arquivo de demonstração não encontrado em: {fixture_path}")
        return

    file_size_kb = fixture_path.stat().st_size / 1024
    print(f"\n[1] Arquivo DBC detectado:")
    print(f"    - Caminho: {fixture_path}")
    print(f"    - Tamanho comprimido (Blast DCL): {file_size_kb:.1f} KB")

    # 2. Descompressão e decodificação 100% nativa em Rust -> Apache Arrow RecordBatch
    print("\n[2] Descomprimindo e decodificando de forma nativa (Zero C/C++ dependencies)...")
    batch = brhealth.read_dbc(str(fixture_path))

    print(f"    -> RecordBatch gerado com sucesso!")
    print(f"    - Total de Linhas:    {batch.num_rows:,}")
    print(f"    - Total de Colunas:   {batch.num_columns}")
    print(f"    - Dimensões (Shape):  {batch.shape}")

    # Amostra dos nomes de colunas
    cols = batch.columns
    print(f"    - Amostra de Colunas: {cols[:10]} ... (+{len(cols) - 10} colunas)")

    # 3. Interoperabilidade Zero-Copy com Polars
    print("\n[3] Conversão Zero-Copy para Polars DataFrame (Arrow C Data Interface):")
    df_polars = batch.to_polars()
    print(f"    - Tipo do Objeto: {type(df_polars).__name__}")
    print(f"    - Shape no Polars: {df_polars.shape}")

    # Selecionar variáveis epidemiológicas chave do SIM
    selected_cols = ["DTOBITO", "IDADE", "SEXO", "RACACOR", "CODMUNRES", "CAUSABAS"]
    available_cols = [c for c in selected_cols if c in df_polars.columns]
    sample_pl = df_polars.select(available_cols).head(5)
    print("\n    Amostra dos Microdados do SIM (Polars):")
    print(sample_pl)

    # 4. Interoperabilidade Zero-Copy com Pandas
    print("\n[4] Conversão Zero-Copy para Pandas DataFrame:")
    df_pandas = batch.to_pandas()
    print(f"    - Tipo do Objeto: {type(df_pandas).__name__}")
    print(f"    - Shape no Pandas: {df_pandas.shape}")
    print(f"    - Uso de Memória: {df_pandas.memory_usage(deep=True).sum() / (1024 * 1024):.2f} MB")

    # 5. Interoperabilidade com PyArrow Table
    print("\n[5] Conversão para PyArrow Table:")
    pa_table = batch.to_pyarrow()
    print(f"    - Tipo do Objeto: {type(pa_table).__name__}")
    print(f"    - Schema Arrow:   {len(pa_table.schema)} campos alinhados a 64 bytes")

    # 6. Descompressão para arquivo .dbf em disco
    output_dbf_path = repo_root / "target" / "doac2022_descomprimido.dbf"
    output_dbf_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"\n[6] Descomprimindo .dbc diretamente para arquivo .dbf canônico:")
    dbf_bytes = brhealth.decompress_dbc(str(fixture_path), output_path=str(output_dbf_path))
    print(f"    - Arquivo gravado em: {output_dbf_path}")
    print(f"    - Tamanho descomprimido: {len(dbf_bytes) / 1024:.1f} KB")
    print(f"    - Taxa de compressão Blast DCL: {file_size_kb / (len(dbf_bytes) / 1024):.2%}")

    # Leitura direta do arquivo DBF recém-gerado
    batch_from_dbf = brhealth.read_dbf(str(output_dbf_path))
    print(f"    - Leitura do DBF gerado: {batch_from_dbf.num_rows:,} linhas, {batch_from_dbf.num_columns} colunas")

    print("\n" + "=" * 70)
    print("  Demonstração concluída com sucesso! (100% Zero-Copy / Native Rust)")
    print("=" * 70)


if __name__ == "__main__":
    main()
