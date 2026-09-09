#!/usr/bin/env python3
# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Script multiplataforma (Linux, macOS, Windows) para limpeza de caches e arquivos temporários do BRHealth.
Não requer dependências externas (apenas a biblioteca padrão do Python 3).
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

# Cores ANSI com detecção multiplataforma
SUPPORTS_COLOR = sys.stdout.isatty() and (
    sys.platform != "win32" or "WT_SESSION" in os.environ or "ANSICON" in os.environ
)

BOLD = "\033[1m" if SUPPORTS_COLOR else ""
GREEN = "\033[0;32m" if SUPPORTS_COLOR else ""
YELLOW = "\033[0;33m" if SUPPORTS_COLOR else ""
CYAN = "\033[0;36m" if SUPPORTS_COLOR else ""
RESET = "\033[0m" if SUPPORTS_COLOR else ""

ROOT_DIR = pathlib.Path(__file__).resolve().parent.parent


def format_size(size_bytes: int) -> str:
    for unit in ["B", "KB", "MB", "GB"]:
        if size_bytes < 1024.0:
            return f"{size_bytes:.1f} {unit}"
        size_bytes /= 1024.0
    return f"{size_bytes:.1f} TB"


def get_dir_size(path: pathlib.Path) -> int:
    total = 0
    try:
        if path.is_file():
            return path.stat().st_size
        for entry in path.rglob("*"):
            if entry.is_file() and not entry.is_symlink():
                total += entry.stat().st_size
    except Exception:
        pass
    return total


def remove_path(path: pathlib.Path) -> int:
    if not path.exists():
        return 0
    size = get_dir_size(path)
    try:
        if path.is_dir():
            shutil.rmtree(path, ignore_errors=True)
        else:
            path.unlink(missing_ok=True)
        print(f"{GREEN}✓ Removido:{RESET} {path} ({format_size(size)})")
        return size
    except Exception as e:
        print(f"{YELLOW}! Falha ao remover {path}: {e}{RESET}")
        return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Limpa de forma multiplataforma todos os caches do BRHealth (Linux, macOS, Windows)."
    )
    parser.add_argument(
        "-c",
        "--cargo",
        action="store_true",
        help="Executa também 'cargo clean' para limpar artefatos de compilação da pasta target/",
    )
    parser.add_argument(
        "-a",
        "--all",
        action="store_true",
        help="Limpa todos os caches e invoca 'cargo clean'",
    )
    args = parser.parse_args()

    clean_cargo = args.cargo or args.all

    print(f"{BOLD}{CYAN}================================================================{RESET}")
    print(f"{BOLD}{CYAN}     BRHealth - Limpeza Multiplataforma de Caches e Temporários {RESET}")
    print(f"{BOLD}{CYAN}     Sistema Operacional: {sys.platform} | Raiz: {ROOT_DIR}{RESET}")
    print(f"{BOLD}{CYAN}================================================================{RESET}\n")

    total_freed = 0

    # 1. Caches temporários do sistema operacional
    print(f"{BOLD}{YELLOW}[1/4] Removendo caches temporários do sistema operacional...{RESET}")
    sys_temp = pathlib.Path(tempfile.gettempdir())
    temp_candidates = [
        sys_temp / "brhealth_cache",
        sys_temp / "brhealth_ffi_cache",
        sys_temp / "brhealth",
    ]

    for cand in temp_candidates:
        total_freed += remove_path(cand)

    # Buscar padrões brhealth* dentro da pasta temp do sistema
    try:
        for p in sys_temp.glob("brhealth*"):
            if p not in temp_candidates and p.exists():
                total_freed += remove_path(p)
    except Exception:
        pass

    # 2. Caches locais do repositório
    print(f"\n{BOLD}{YELLOW}[2/4] Removendo caches analíticos locais...{RESET}")
    local_candidates = [
        ROOT_DIR / ".brhealth_cache",
        ROOT_DIR / "data" / "cache",
        ROOT_DIR / "target" / "hive_cache",
        ROOT_DIR / "cache",
    ]

    for cand in local_candidates:
        total_freed += remove_path(cand)

    # 3. Caches de ambiente Python
    print(f"\n{BOLD}{YELLOW}[3/4] Removendo caches Python (__pycache__, .pytest_cache)...{RESET}")
    py_dirs = ["__pycache__", ".pytest_cache", ".ruff_cache", ".mypy_cache"]
    for dir_name in py_dirs:
        for p in ROOT_DIR.rglob(dir_name):
            if p.is_dir():
                total_freed += remove_path(p)

    for ext in ["*.pyc", "*.pyo", ".coverage"]:
        for f in ROOT_DIR.rglob(ext):
            if f.is_file():
                total_freed += remove_path(f)

    # 4. Cargo Clean opcional
    if clean_cargo:
        print(f"\n{BOLD}{YELLOW}[4/4] Executando 'cargo clean'...{RESET}")
        target_dir = ROOT_DIR / "target"
        if target_dir.exists():
            target_size = get_dir_size(target_dir)
            try:
                subprocess.run(["cargo", "clean"], cwd=ROOT_DIR, check=True)
                total_freed += target_size
                print(f"{GREEN}✓ 'cargo clean' concluído com sucesso ({format_size(target_size)} liberados).{RESET}")
            except Exception as e:
                print(f"{YELLOW}! Falha ao executar 'cargo clean': {e}{RESET}")
    else:
        print(f"\n{BOLD}{YELLOW}[4/4] Artefatos do Cargo (target/) preservados.{RESET}")
        print(f"{CYAN}(Use '--cargo' para limpar também a pasta target/){RESET}")

    print(f"\n{BOLD}{CYAN}================================================================{RESET}")
    print(f"{BOLD}{GREEN}        ✓ LIMPEZA CONCLUÍDA: {format_size(total_freed)} liberados!{RESET}")
    print(f"{BOLD}{CYAN}================================================================{RESET}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
