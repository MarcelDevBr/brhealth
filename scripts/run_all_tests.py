#!/usr/bin/env python3
# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Executor mestre multiplataforma (Linux, macOS, Windows) de toda a suíte de testes do BRHealth.
Não requer dependências externas (apenas a biblioteca padrão do Python 3).
"""

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
RESET = "\033[0m" if SUPPORTS_COLOR else ""

ROOT_DIR = pathlib.Path(__file__).resolve().parent.parent


def run_step(title: str, cmd: list[str]) -> bool:
    print(f"\n{BOLD}{YELLOW}[ETAPA] {title}{RESET}")
    print(f"{CYAN}Comando: {' '.join(cmd)}{RESET}")

    start = time.perf_counter()
    res = subprocess.run(cmd, cwd=ROOT_DIR)
    elapsed = time.perf_counter() - start

    if res.returncode == 0:
        print(f"{GREEN}✓ {title} concluído com sucesso ({elapsed:.2f}s){RESET}")
        return True
    else:
        print(f"{RED}✗ {title} FALHOU com código de retorno {res.returncode} ({elapsed:.2f}s){RESET}")
        return False


def main() -> int:
    print(f"{BOLD}{CYAN}================================================================{RESET}")
    print(f"{BOLD}{CYAN}   BRHealth - Execução Completa da Suíte de Testes (Multiplataforma)  {RESET}")
    print(f"{BOLD}{CYAN}   Sistema: {sys.platform} | Raiz: {ROOT_DIR}{RESET}")
    print(f"{BOLD}{CYAN}================================================================{RESET}")

    steps = [
        (
            "Verificação Estática de Tipos (cargo check)",
            ["cargo", "check", "--workspace", "--all-targets"],
        ),
        (
            "Linter de Alta Rigidez (cargo clippy)",
            ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ),
        (
            "Testes de Unidade e Integração (Core, FFI, JNI, CLI)",
            ["cargo", "test", "-p", "brhealth-core", "-p", "brhealth-ffi", "-p", "brhealth-jni", "-p", "brhealth-cli"],
        ),
        (
            "Testes do Módulo Python (PyO3 & PyCapsule)",
            ["cargo", "test", "-p", "brhealth-python"],
        ),
        (
            "Doc-tests da Documentação Formal (LaTeX)",
            ["cargo", "test", "--doc"],
        ),
        (
            "Validação Granular das 26 Fontes Oficiais",
            ["cargo", "test", "--test", "test_each_source"],
        ),
        (
            "Testes da Interface de Linha de Comando (CLI)",
            ["cargo", "test", "--test", "test_cli"],
        ),
    ]

    total_start = time.perf_counter()
    failed = 0

    for title, cmd in steps:
        if not run_step(title, cmd):
            failed += 1

    total_time = time.perf_counter() - total_start

    print(f"\n{BOLD}{CYAN}================================================================{RESET}")
    if failed == 0:
        print(f"{BOLD}{GREEN}        ✓ TODOS OS TESTES FORAM APROVADOS COM SUCESSO! ({total_time:.2f}s)   {RESET}")
        print(f"{BOLD}{CYAN}================================================================{RESET}")
        return 0
    else:
        print(f"{BOLD}{RED}        ✗ {failed} ETAPA(S) DE TESTE FALHARAM! ({total_time:.2f}s)               {RESET}")
        print(f"{BOLD}{CYAN}================================================================{RESET}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
