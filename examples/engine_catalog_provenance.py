# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Catálogo Federado, Motor Analítico (Engine) e Governança FAIR:
1. Verificação diagnóstica do ecossistema analítico (`check_environment()`).
2. Inicialização do motor analítico central `brhealth.Engine()`.
3. Inspeção e categorização do catálogo de fontes de dados de saúde (26 fontes federadas):
   - DATASUS (SIM, SIH, SINASC, SINAN, SIA, CNES, SISVAN, SIPNI, SISCAN)
   - IBGE (Censo, POF, PNAD Contínua)
   - Meio Ambiente e Clima (INMET, BDQueimadas, Prodes, Siságua)
   - Vulnerabilidade Social (MDS CadÚnico)
   - Agências Internacionais (WHO GHO, Copernicus ERA5, OpenAQ, IHME GBD, WorldPop)
4. Acessores Semânticos por Domínio:
   - `engine.hospital_morbidity`
   - `engine.vital_statistics`
   - `engine.notifications`
   - `engine.demographics`
   - `engine.ambulatory`
   - `engine.environmental`
   - `engine.social`
   - `engine.global_climate`
5. Governança do Cache Hive-Parquet (`engine.cache`).
6. Princípios FAIR e Linhagem Criptográfica W3C PROV-O com Hashes SHA-256.
"""

import brhealth
from brhealth import Engine, check_environment


def main() -> None:
    print("=" * 80)
    print("  BRHealth: Catálogo Federado de Fontes, Engine e Governança de Dados")
    print("=" * 80)

    # 1. Diagnóstico do Ambiente
    print("\n[1] Diagnóstico de Bibliotecas e Interoperabilidade Zero-Copy:")
    env = check_environment()
    for lib, status in env.items():
        icone = "✓ DISPONÍVEL" if status else "✗ AUSENTE    "
        print(f"    - [{icone}] {lib:<12}")

    # 2. Inicialização do Motor Central
    print("\n[2] Inicializando o BRHealth Engine...")
    engine = Engine()
    print(f"    - Versão do Engine: {engine.version()}")
    print(f"    - Total de Fontes Registradas: {engine.source_count()}")

    # 3. Catálogo de Fontes Registradas por Domínio
    fontes = engine.list_sources()
    categorias = {
        "DATASUS / Ministério da Saúde": [s for s in fontes if s.startswith("datasus.")],
        "IBGE / Demografia": [s for s in fontes if s.startswith("ibge.")],
        "Determinantes Ambientais e Clima": [s for s in fontes if s.startswith("environmental.") or "inmet" in s],
        "Vulnerabilidade Social (MDS)": [s for s in fontes if s.startswith("mds.")],
        "Agências Globais e Internacionais": [s for s in fontes if s.startswith("global.")],
    }

    print("\n[3] Catálogo Federado de Fontes de Dados de Saúde:")
    for categoria, lista_fontes in categorias.items():
        print(f"\n    • {categoria} ({len(lista_fontes)} fontes):")
        for fonte in sorted(lista_fontes):
            print(f"      - {fonte}")

    # 4. Demonstração dos Acessores Especializados
    print("\n[4] Acessores Especializados por Domínio (Interface Semântica):")
    accessors = [
        ("hospital_morbidity", engine.hospital_morbidity, "Morbidade Hospitalar do SUS (SIH/AIH)"),
        ("vital_statistics", engine.vital_statistics, "Estatísticas Vitais (Mortalidade SIM e Nascidos Vivos SINASC)"),
        ("notifications", engine.notifications, "Vigilância de Agravos e Doenças de Notificação Compulsória (SINAN)"),
        ("demographics", engine.demographics, "Censos e Projeções Populacionais Municipais (IBGE)"),
        ("ambulatory", engine.ambulatory, "Produção Ambulatorial (SIA) e Estabelecimentos (CNES)"),
        ("environmental", engine.environmental, "Qualidade da Água (Siságua), Queimadas e Monitoramento"),
        ("social", engine.social, "Cadastro Único para Programas Sociais (CadÚnico / MDS)"),
        ("global_climate", engine.global_climate, "Reanálise Climática Global (Copernicus ERA5)"),
    ]

    for nome, accessor, descricao in accessors:
        tipo = type(accessor).__name__
        print(f"    - engine.{nome:<18} -> {tipo:<26} | {descricao}")

    # 5. Gestão e Governança do Cache Hive-Parquet
    print("\n[5] Governança de Armazenamento e Cache Hive-Parquet:")
    cache_mgr = engine.cache
    status_cache = cache_mgr.status()

    tamanho_mb = status_cache["total_bytes"] / (1024 * 1024)
    print(f"    - Diretório Base do Cache:   {status_cache['base_path']}")
    print(f"    - Volume Ocupado em Disco:   {tamanho_mb:.2f} MB ({status_cache['total_bytes']:,} bytes)")
    print(f"    - Total de Snapshots Salvos: {status_cache['snapshot_count']}")

    # Estratégia de limpeza controlada por tempo de retenção
    removidos = cache_mgr.clear_older_than(days=90)
    print(f"    - Snapshots expirados (> 90 dias) removidos: {removidos}")

    # 6. Rastreabilidade Criptográfica e Padrões FAIR (W3C PROV-O)
    print("\n[6] Linhagem e Reprodutibilidade Científica (Padrões FAIR):")
    print("    - Toda ingestão via `engine.fetch()` gera automaticamente:")
    print("      1. Hashes SHA-256 criptográficos de cada arquivo bruto ingerido.")
    print("      2. Grafo de proveniência W3C PROV-O serializável em JSON-LD.")
    print("      3. Carimbos temporais UTC imutáveis e versionamento de ontologias (CID-10, IBGE, SIGTAP).")
    print("      4. Suporte a Time-Travel sobre a camada colunar Apache Parquet particionada em Hive.")

    print("\n" + "=" * 80)
    print("  Demonstração do Catálogo e Arquitetura do Engine concluída com sucesso!")
    print("=" * 80)


if __name__ == "__main__":
    main()
