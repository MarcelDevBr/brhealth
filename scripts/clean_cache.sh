#!/usr/bin/env bash
# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

set -euo pipefail

# Cores e formatação
BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[0;33m"
CYAN="\033[0;36m"
RESET="\033[0m"

# Diretório raiz do projeto
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

CLEAN_CARGO=false
FORCE=false

print_help() {
    echo -e "${BOLD}Uso:${RESET} ./scripts/clean_cache.sh [OPÇÕES]"
    echo -e "\n${BOLD}Opções:${RESET}"
    echo -e "  -c, --cargo      Executa também 'cargo clean' para limpar a pasta target/ de compilação"
    echo -e "  -a, --all        Limpa todos os caches temporários e executa 'cargo clean'"
    echo -e "  -f, --force      Executa a limpeza sem confirmações interativas"
    echo -e "  -h, --help       Exibe esta mensagem de ajuda"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -c|--cargo)
            CLEAN_CARGO=true
            shift
            ;;
        -a|--all)
            CLEAN_CARGO=true
            shift
            ;;
        -f|--force)
            FORCE=true
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            echo -e "${RED}Opção desconhecida: $1${RESET}"
            print_help
            exit 1
            ;;
    esac
done

echo -e "${BOLD}${CYAN}================================================================${RESET}"
echo -e "${BOLD}${CYAN}            BRHealth - Limpeza de Caches e Temporários           ${RESET}"
echo -e "${BOLD}${CYAN}================================================================${RESET}"

# 1. Caches temporários do sistema (/tmp/brhealth*)
echo -e "\n${BOLD}${YELLOW}[1/4] Removendo caches temporários do sistema operacional...${RESET}"
TMP_TARGETS=(
    "/tmp/brhealth_cache"
    "/tmp/brhealth_ffi_cache"
    "/tmp/brhealth"
)

for target in "${TMP_TARGETS[@]}"; do
    if [ -d "${target}" ] || [ -f "${target}" ]; then
        SIZE=$(du -sh "${target}" 2>/dev/null | cut -f1 || echo "0B")
        rm -rf "${target}"
        echo -e "${GREEN}✓ Removido:${RESET} ${target} (${SIZE})"
    fi
done

# Remoção de padrões curinga em /tmp com segurança
find /tmp -maxdepth 1 -name "brhealth*" -exec rm -rf {} + 2>/dev/null || true
echo -e "${GREEN}✓ Caches em /tmp limpos com sucesso.${RESET}"

# 2. Caches locais do repositório (Hive-Parquet e downloads locais)
echo -e "\n${BOLD}${YELLOW}[2/4] Removendo caches analíticos locais...${RESET}"
LOCAL_TARGETS=(
    "${ROOT_DIR}/.brhealth_cache"
    "${ROOT_DIR}/data/cache"
    "${ROOT_DIR}/target/hive_cache"
    "${ROOT_DIR}/cache"
)

for target in "${LOCAL_TARGETS[@]}"; do
    if [ -d "${target}" ] || [ -f "${target}" ]; then
        SIZE=$(du -sh "${target}" 2>/dev/null | cut -f1 || echo "0B")
        rm -rf "${target}"
        echo -e "${GREEN}✓ Removido:${RESET} ${target} (${SIZE})"
    fi
done

echo -e "${GREEN}✓ Caches analíticos locais em Parquet limpos.${RESET}"

# 3. Caches de ambiente Python
echo -e "\n${BOLD}${YELLOW}[3/4] Removendo caches de Python e testes...${RESET}"
PYTHON_CACHE_COUNT=0
while IFS= read -r -d '' dir; do
    rm -rf "${dir}"
    PYTHON_CACHE_COUNT=$((PYTHON_CACHE_COUNT + 1))
done < <(find "${ROOT_DIR}" \( -name "__pycache__" -o -name ".pytest_cache" -o -name ".ruff_cache" -o -name ".mypy_cache" \) -print0 2>/dev/null)

find "${ROOT_DIR}" -type f \( -name "*.pyc" -o -name "*.pyo" -o -name ".coverage" \) -delete 2>/dev/null || true
echo -e "${GREEN}✓ ${PYTHON_CACHE_COUNT} diretórios de cache Python limpos.${RESET}"

# 4. Limpeza opcional do Cargo Target
if [ "${CLEAN_CARGO}" = true ]; then
    echo -e "\n${BOLD}${YELLOW}[4/4] Limpando artefatos de compilação do Cargo (cargo clean)...${RESET}"
    if [ -d "${ROOT_DIR}/target" ]; then
        TARGET_SIZE=$(du -sh "${ROOT_DIR}/target" 2>/dev/null | cut -f1 || echo "desconhecido")
        echo -e "${CYAN}Tamanho do diretório target/: ${TARGET_SIZE}${RESET}"
        cargo clean
        echo -e "${GREEN}✓ 'cargo clean' concluído com sucesso.${RESET}"
    else
        echo -e "${CYAN}Diretório target/ já se encontra limpo.${RESET}"
    fi
else
    echo -e "\n${BOLD}${YELLOW}[4/4] Artefatos do Cargo (target/) preservados.${RESET}"
    echo -e "${CYAN}(Para limpar o target do cargo, use: ./scripts/clean_cache.sh --cargo)${RESET}"
fi

echo -e "\n${BOLD}${CYAN}================================================================${RESET}"
echo -e "${BOLD}${GREEN}        ✓ LIMPEZA DE CACHE CONCLUÍDA COM SUCESSO!               ${RESET}"
echo -e "${BOLD}${CYAN}================================================================${RESET}"
