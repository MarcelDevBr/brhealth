#!/usr/bin/env python3
# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Script multiplataforma (Linux, macOS, Windows) para teste das 26 fontes de dados do BRHealth.
Permite testar todas as fontes, por grupo ou individualmente.
"""

import argparse
import os
import pathlib
import subprocess
import sys
import time

SUPPORTS_COLOR = sys.stdout.isatty() and (
    sys.platform != "win32" or "WT_SESSION" in os.environ or "ANSICON" in os.environ
)

BOLD = "\033[1m" if SUPPORTS_COLOR else ""
GREEN = "\033[0;32m" if SUPPORTS_COLOR else ""
RED = "\033[0;31m" if SUPPORTS_COLOR else ""
YELLOW = "\033[0;33m" if SUPPORTS_COLOR else ""
CYAN = "\033[0;36m" if SUPPORTS_COLOR else ""
MAGENTA = "\033[0;35m" if SUPPORTS_COLOR else ""
RESET = "\033[0m" if SUPPORTS_COLOR else ""

ROOT_DIR = pathlib.Path(__file__).resolve().parent.parent

# Catálogo canônico das 26 fontes do BRHealth: (função de teste, ID formal, Nome amigável, Grupo)
SOURCES = [
    # DATASUS (10)
    ("datasus_sim", "datasus.sim", "Mortalidade (SIM)", "datasus"),
    ("datasus_sinasc", "datasus.sinasc", "Nascidos Vivos (SINASC)", "datasus"),
    ("datasus_sih", "datasus.sih", "Internações Hospitalares (SIH/RD)", "datasus"),
    ("datasus_sinan", "datasus.sinan", "Agravos de Notificação (SINAN)", "datasus"),
    ("datasus_siasus", "datasus.siasus", "Ambulatorial (SIA-SUS)", "datasus"),
    ("datasus_cnes", "datasus.cnes", "Estabelecimentos de Saúde (CNES)", "datasus"),
    ("datasus_sipni", "datasus.sipni", "Imunizações (SI-PNI/RNDS)", "datasus"),
    ("datasus_sisvan", "datasus.sisvan", "Vigilância Alimentar e Nutricional (SISVAN)", "datasus"),
    ("datasus_siscan", "datasus.siscan", "Câncer / Mamografia / Colo (SISCAN)", "datasus"),
    ("datasus_bps", "datasus.bps", "Preços em Saúde e Fármacos (BPS/CMED)", "datasus"),
    # IBGE (5)
    ("ibge_censo", "ibge.censo", "Censo Demográfico", "ibge"),
    ("ibge_pnad", "ibge.pnad", "PNAD Contínua", "ibge"),
    ("ibge_pof", "ibge.pof", "Pesquisa de Orçamentos Familiares (POF)", "ibge"),
    ("ibge_pense", "ibge.pense", "Pesquisa Nacional de Saúde do Escolar (PeNSE)", "ibge"),
    ("ibge_munic", "ibge.munic", "Pesquisa de Informações Básicas Municipais (MUNIC)", "ibge"),
    # MDS (1)
    ("mds_cadunico", "mds.cadunico", "Cadastro Único para Programas Sociais (CadÚnico)", "mds"),
    # AMBIENTAL E CLIMA (4)
    ("environmental_inmet", "environmental.inmet", "Estações Meteorológicas (INMET)", "environmental"),
    ("environmental_bdqueimadas", "environmental.bdqueimadas", "Focos de Queimadas (INPE/BDQueimadas)", "environmental"),
    ("environmental_prodes", "environmental.prodes", "Monitoramento de Desmatamento (INPE/PRODES)", "environmental"),
    ("environmental_sisagua", "environmental.sisagua", "Qualidade da Água para Consumo (SISAGUA)", "environmental"),
    # GLOBAL / SUPRANACIONAL (6)
    ("global_who_gho", "global.who_gho", "World Health Organization Global Health Observatory (WHO GHO)", "global"),
    ("global_ihme_gbd", "global.ihme_gbd", "Global Burden of Disease (IHME GBD)", "global"),
    ("global_copernicus_era5", "global.copernicus_era5", "Copernicus ERA5-Land Reanalysis", "global"),
    ("global_worldpop", "global.worldpop", "WorldPop High Resolution Population Mapping", "global"),
    ("global_paho_plisa", "global.paho_plisa", "PAHO/OPAS Plataforma de Informação de Saúde das Américas", "global"),
    ("global_openaq", "global.openaq", "OpenAQ Global Air Quality Platform", "global"),
]


def run_single_test(test_fn: str, source_id: str, source_name: str) -> bool:
    sys.stdout.write(f"  {source_id:<28} ")
    sys.stdout.flush()

    cmd = [
        "cargo",
        "test",
        "-q",
        "--test",
        "test_each_source",
        f"test_source_{test_fn}",
        "--",
        "--exact",
    ]

    start = time.perf_counter()
    res = subprocess.run(
        cmd,
        cwd=ROOT_DIR,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    elapsed_ms = int((time.perf_counter() - start) * 1000)

    if res.returncode == 0:
        print(f"{GREEN}✓ APROVADO{RESET} ({elapsed_ms}ms) - {source_name}")
        return True
    else:
        print(f"{RED}✗ FALHOU{RESET} - {source_name}")
        print(f"{RED}{res.stdout}{RESET}")
        return False


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Executor multiplataforma de testes das 26 fontes oficiais do BRHealth."
    )
    parser.add_argument(
        "-l", "--list", action="store_true", help="Lista todas as 26 fontes oficiais cadastradas"
    )
    parser.add_argument(
        "-a", "--all", action="store_true", help="Executa o teste de todas as 26 fontes oficiais"
    )
    parser.add_argument(
        "-p",
        "--pack",
        type=str,
        help="Executa testes para um grupo específico: brasil, global, datasus, ibge, mds, environmental",
    )
    parser.add_argument(
        "source",
        nargs="?",
        help="Nome ou ID da fonte a ser testada (ex: datasus.sim, censo, era5, sih)",
    )

    args = parser.parse_args()

    if args.list:
        print(f"{BOLD}{CYAN}=== Catálogo Oficial das 26 Fontes do BRHealth ==={RESET}\n")
        groups = {
            "datasus": "Fontes Nacionais - DATASUS (10)",
            "ibge": "Fontes Nacionais - IBGE (5)",
            "mds": "Fontes Nacionais - MDS (1)",
            "environmental": "Fontes Nacionais - Ambiental e Clima (4)",
            "global": "Fontes Supranacionais - Pack Global (6)",
        }
        for grp_key, grp_title in groups.items():
            print(f"{BOLD}{MAGENTA}▶ {grp_title}{RESET}")
            for fn, s_id, s_name, g in SOURCES:
                if g == grp_key:
                    print(f"  {s_id:<28} {s_name}")
            print()
        return 0

    to_test = []

    if args.all:
        to_test = SOURCES
    elif args.pack:
        pack_lower = args.pack.lower()
        if pack_lower in ["brasil", "br", "national"]:
            to_test = [s for s in SOURCES if s[3] in ["datasus", "ibge", "mds", "environmental"]]
        elif pack_lower in ["global", "supranational"]:
            to_test = [s for s in SOURCES if s[3] == "global"]
        else:
            to_test = [s for s in SOURCES if s[3] == pack_lower]
        if not to_test:
            print(f"{RED}Erro: Pacote desconhecido '{args.pack}'. Use: brasil, global, datasus, ibge, mds, environmental{RESET}")
            return 1
    elif args.source:
        query = args.source.lower()
        to_test = [s for s in SOURCES if query in s[1].lower() or query in s[0].lower()]
        if not to_test:
            print(f"{RED}Nenhuma fonte encontrada com o termo '{args.source}'. Use --list para ver as opções.{RESET}")
            return 1
    else:
        parser.print_help()
        return 0

    print(f"{BOLD}{CYAN}================================================================{RESET}")
    print(f"{BOLD}{CYAN}      BRHealth - Teste de Fontes Analíticas ({len(to_test)} selecionadas)       {RESET}")
    print(f"{BOLD}{CYAN}      Plataforma: {sys.platform} | Raiz: {ROOT_DIR}{RESET}")
    print(f"{BOLD}{CYAN}================================================================{RESET}\n")

    failed = 0
    start_total = time.perf_counter()

    for fn, s_id, s_name, _ in to_test:
        if not run_single_test(fn, s_id, s_name):
            failed += 1

    total_sec = time.perf_counter() - start_total

    print(f"\n{BOLD}{CYAN}================================================================{RESET}")
    if failed == 0:
        print(f"{BOLD}{GREEN}        ✓ TODAS AS {len(to_test)} FONTES FORAM APROVADAS! ({total_sec:.2f}s)       {RESET}")
        print(f"{BOLD}{CYAN}================================================================{RESET}")
        return 0
    else:
        print(f"{BOLD}{RED}        ✗ FALHA NO TESTE DE {failed} FONTE(S)! ({total_sec:.2f}s)               {RESET}")
        print(f"{BOLD}{CYAN}================================================================{RESET}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
