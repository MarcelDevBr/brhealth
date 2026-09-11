# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Ingestão Automatizada, Política Cache-First e Zero-Copy:
1. Ingestão automatizada com política Cache-First:
   - "Se tem cache usa": Lê diretamente do cache local Hive-Parquet (~/.brhealth/cache) em Zero-Copy.
   - "Senão baixa da fonte": Conecta ao DATASUS, descarrega, descomprime com Blast DCL e grava em cache.
2. Ingestão semântica via `engine.vital_statistics.fetch(...)`.
3. Travessia Zero-Copy para Polars, Pandas e PyArrow via Arrow C Data Interface.
4. Inspeção analítica e bioestatística de microdados reais de mortalidade (SIM-SUS).
"""

import warnings
import brhealth

# Suprimir avisos transitórios de migração de bibliotecas
warnings.filterwarnings("ignore", category=FutureWarning)


def main() -> None:
    print("=" * 75)
    print("  BRHealth: Ingestão Automatizada, Cache-First e Zero-Copy (Python API)")
    print("=" * 75)

    # 1. Ingestão Canônica Automatizada (Cache-First)
    print("\n[1] Ingestão Automatizada via brhealth.fetch() (Política Cache-First):")
    print("    - Fonte Solicitada: SIM (Mortalidade)")
    print("    - Jurisdição:       AC (Acre)")
    print("    - Ano de Exercício: 2022")
    print("    -> O sistema verifica o cache Hive-Parquet. Se ausente, baixa da fonte oficial.")

    batch = brhealth.fetch("datasus.sim", jurisdiction="AC", year=2022)

    print("    ✓ Lote obtido com sucesso!")
    print(f"    - Total de Linhas:    {batch.num_rows:,}")
    print(f"    - Total de Colunas:   {batch.num_columns}")
    print(f"    - Dimensões (Shape):  {batch.shape}")
    print(f"    - Manifesto Criptográfico FAIR presente: {batch.manifest_json is not None}")

    # 2. Ingestão Semântica via Engine Domain Accessor
    print("\n[2] Ingestão Semântica via engine.vital_statistics.fetch():")
    print("    -> Acesso tipado e canônico ao subsistema de Estatísticas Vitais (SIM):")
    engine = brhealth.Engine()
    batch_semantic = engine.vital_statistics.fetch(source="SIM", jurisdiction="AC", year=2022)
    print(f"    ✓ Carregado via engine: {batch_semantic.num_rows:,} linhas x {batch_semantic.num_columns} colunas")

    # 3. Interoperabilidade Zero-Copy com Polars
    print("\n[3] Conversão Zero-Copy para Polars DataFrame (Arrow C Data Interface):")
    df_polars = batch.to_polars()
    print(f"    - Tipo do Objeto: {type(df_polars).__name__}")
    print(f"    - Shape no Polars: {df_polars.shape}")

    # Amostra dos dados do SIM
    amostra_cols = [c for c in ["record_id", "date", "diagnosis_icd10", "age_years", "sex", "race_ethnicity"] if c in df_polars.columns]
    if not amostra_cols:
        amostra_cols = df_polars.columns[:6]

    print("\n    Amostra dos Dados Canônicos (Polars):")
    print(df_polars.select(amostra_cols).head(5))

    # 4. Interoperabilidade Zero-Copy com Pandas
    print("\n[4] Conversão Zero-Copy para Pandas DataFrame:")
    df_pandas = batch.to_pandas()
    print(f"    - Tipo do Objeto: {type(df_pandas).__name__}")
    print(f"    - Shape no Pandas: {df_pandas.shape}")
    print(f"    - Uso de Memória: {df_pandas.memory_usage(deep=True).sum() / 1024:.1f} KB")

    # 5. Interoperabilidade com PyArrow Table
    print("\n[5] Conversão para PyArrow Table:")
    pa_table = batch.to_pyarrow()
    print(f"    - Tipo do Objeto: {type(pa_table).__name__}")
    print(f"    - Schema Arrow:   {len(pa_table.schema)} campos alinhados a 64 bytes")

    # 6. Status do Cache Local Hive-Parquet
    print("\n[6] Status da Camada de Cache Hive-Parquet:")
    engine = brhealth.Engine()
    cache_status = engine.cache.status()
    print(f"    - Diretório do Cache:   {cache_status['base_path']}")
    print(f"    - Volume em Disco:      {cache_status['total_bytes'] / 1024:.1f} KB")
    print(f"    - Snapshots Gravados:   {cache_status['snapshot_count']}")

    print("\n" + "=" * 75)
    print("  Demonstração concluída com sucesso! (Automação Total / Cache-First)")
    print("=" * 75)


if __name__ == "__main__":
    main()
