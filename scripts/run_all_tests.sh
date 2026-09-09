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

echo -e "${BOLD}${CYAN}================================================================${RESET}"
echo -e "${BOLD}${CYAN}        BRHealth - Execução Completa da Suíte de Testes         ${RESET}"
echo -e "${BOLD}${CYAN}================================================================${RESET}"
echo -e "Raiz do projeto: ${ROOT_DIR}"
echo -e "Data / Hora:     $(date -u +"%Y-%m-%dT%H:%M:%SZ")\n"

START_TIME=$(date +%s)
FAILED=0

run_step() {
    local step_name="$1"
    shift
    echo -e "\n${BOLD}${YELLOW}[ETAPA] ${step_name}${RESET}"
    echo -e "${CYAN}Comando: $*${RESET}"
    
    local step_start=$(date +%s)
    if "$@"; then
        local step_end=$(date +%s)
        local duration=$((step_end - step_start))
        echo -e "${GREEN}✓ ${step_name} concluído com sucesso (${duration}s)${RESET}"
    else
        local step_end=$(date +%s)
        local duration=$((step_end - step_start))
        echo -e "${RED}✗ ${step_name} FALHOU (${duration}s)${RESET}"
        FAILED=1
    fi
}

# 1. Compilação e Verificação Estática de Tipos
run_step "Verificação de Tipos (cargo check --workspace --all-targets)" \
    cargo check --workspace --all-targets

# 2. Linter Estrito (Zero Warnings)
run_step "Análise Estática de Qualidade (cargo clippy --workspace --all-targets -- -D warnings)" \
    cargo clippy --workspace --all-targets -- -D warnings

# 3. Testes Unitários e de Integração dos Cratas Principais
run_step "Testes de Core, FFI, JNI e CLI" \
    cargo test -p brhealth-core -p brhealth-ffi -p brhealth-jni -p brhealth-cli

# 4. Testes do Módulo de Bindings Python
run_step "Testes de Integração Python (PyO3 & Arrow PyCapsule)" \
    cargo test -p brhealth-python

# 5. Doc-tests Executáveis
run_step "Doc-tests da Documentação Formal (LaTeX & Formulas)" \
    cargo test --doc

# 6. Teste de Todas as 26 Fontes Oficiais Individuais
run_step "Validação Granular das 26 Fontes Oficiais (test_each_source)" \
    cargo test --test test_each_source

# 7. Testes da CLI Binária em Ação
run_step "Validação dos Subcomandos da CLI Nativa" \
    cargo test --test test_cli

END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))

echo -e "\n${BOLD}${CYAN}================================================================${RESET}"
if [ $FAILED -eq 0 ]; then
    echo -e "${BOLD}${GREEN}        ✓ TODOS OS TESTES FORAM APROVADOS COM SUCESSO!         ${RESET}"
    echo -e "${BOLD}${GREEN}        Tempo Total de Execução: ${TOTAL_DURATION}s            ${RESET}"
    echo -e "${BOLD}${CYAN}================================================================${RESET}"
    exit 0
else
    echo -e "${BOLD}${RED}        ✗ UMA OU MAIS ETAPAS DE TESTE FALHARAM!                 ${RESET}"
    echo -e "${BOLD}${RED}        Tempo Total: ${TOTAL_DURATION}s                        ${RESET}"
    echo -e "${BOLD}${CYAN}================================================================${RESET}"
    exit 1
fi
