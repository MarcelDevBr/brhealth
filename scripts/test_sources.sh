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
MAGENTA="\033[0;35m"
RESET="\033[0m"

# Diretório raiz do projeto
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

# Lista canônica das 26 fontes oficiais suportadas pelo BRHealth
DATASUS_SOURCES=(
    "datasus_sim:datasus.sim:Mortalidade (SIM)"
    "datasus_sinasc:datasus.sinasc:Nascidos Vivos (SINASC)"
    "datasus_sih:datasus.sih:Internações Hospitalares (SIH/RD)"
    "datasus_sinan:datasus.sinan:Agravos de Notificação (SINAN)"
    "datasus_siasus:datasus.siasus:Ambulatorial (SIA-SUS)"
    "datasus_cnes:datasus.cnes:Estabelecimentos de Saúde (CNES)"
    "datasus_sipni:datasus.sipni:Imunizações (SI-PNI/RNDS)"
    "datasus_sisvan:datasus.sisvan:Vigilância Alimentar e Nutricional (SISVAN)"
    "datasus_siscan:datasus.siscan:Câncer / Mamografia / Colo (SISCAN)"
    "datasus_bps:datasus.bps:Preços em Saúde e Fármacos (BPS/CMED)"
)

IBGE_SOURCES=(
    "ibge_censo:ibge.censo:Censo Demográfico"
    "ibge_pnad:ibge.pnad:PNAD Contínua"
    "ibge_pof:ibge.pof:Pesquisa de Orçamentos Familiares (POF)"
    "ibge_pense:ibge.pense:Pesquisa Nacional de Saúde do Escolar (PeNSE)"
    "ibge_munic:ibge.munic:Pesquisa de Informações Básicas Municipais (MUNIC)"
)

MDS_SOURCES=(
    "mds_cadunico:mds.cadunico:Cadastro Único para Programas Sociais (CadÚnico)"
)

ENVIRONMENTAL_SOURCES=(
    "environmental_inmet:environmental.inmet:Estações Meteorológicas (INMET)"
    "environmental_bdqueimadas:environmental.bdqueimadas:Focos de Queimadas (INPE/BDQueimadas)"
    "environmental_prodes:environmental.prodes:Monitoramento de Desmatamento (INPE/PRODES)"
    "environmental_sisagua:environmental.sisagua:Qualidade da Água para Consumo (SISAGUA)"
)

GLOBAL_SOURCES=(
    "global_who_gho:global.who_gho:World Health Organization Global Health Observatory (WHO GHO)"
    "global_ihme_gbd:global.ihme_gbd:Global Burden of Disease (IHME GBD)"
    "global_copernicus_era5:global.copernicus_era5:Copernicus ERA5-Land Reanalysis"
    "global_worldpop:global.worldpop:WorldPop High Resolution Population Mapping"
    "global_paho_plisa:global.paho_plisa:PAHO/OPAS Plataforma de Informação de Saúde das Américas"
    "global_openaq:global.openaq:OpenAQ Global Air Quality Platform"
)

print_help() {
    echo -e "${BOLD}Uso:${RESET} ./scripts/test_sources.sh [OPÇÕES | FONTE]"
    echo -e "\n${BOLD}Exemplos de uso:${RESET}"
    echo -e "  ./scripts/test_sources.sh --all                Testa todas as 26 fontes oficiais"
    echo -e "  ./scripts/test_sources.sh --list               Lista todas as 26 fontes disponíveis"
    echo -e "  ./scripts/test_sources.sh --pack brasil        Testa todas as 20 fontes nacionais do Pack Brasil"
    echo -e "  ./scripts/test_sources.sh --pack global        Testa as 6 fontes do Pack Global"
    echo -e "  ./scripts/test_sources.sh --pack datasus       Testa as 10 fontes do DATASUS"
    echo -e "  ./scripts/test_sources.sh --pack ibge          Testa as 5 fontes do IBGE"
    echo -e "  ./scripts/test_sources.sh --pack environmental Testa as 4 fontes ambientais/climáticas"
    echo -e "  ./scripts/test_sources.sh datasus.sim          Testa a fonte individual SIM"
    echo -e "  ./scripts/test_sources.sh ibge.censo           Testa a fonte individual IBGE Censo"
    echo -e "  ./scripts/test_sources.sh era5                 Testa a fonte individual Copernicus ERA5"
}

list_sources() {
    echo -e "${BOLD}${CYAN}=== Fontes Nacionais - DATASUS (10 fontes) ===${RESET}"
    for item in "${DATASUS_SOURCES[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        printf "  %-26s %s\n" "${id}" "${name}"
    done

    echo -e "\n${BOLD}${CYAN}=== Fontes Nacionais - IBGE (5 fontes) ===${RESET}"
    for item in "${IBGE_SOURCES[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        printf "  %-26s %s\n" "${id}" "${name}"
    done

    echo -e "\n${BOLD}${CYAN}=== Fontes Nacionais - MDS (1 fonte) ===${RESET}"
    for item in "${MDS_SOURCES[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        printf "  %-26s %s\n" "${id}" "${name}"
    done

    echo -e "\n${BOLD}${CYAN}=== Fontes Nacionais - Meio Ambiente e Clima (4 fontes) ===${RESET}"
    for item in "${ENVIRONMENTAL_SOURCES[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        printf "  %-26s %s\n" "${id}" "${name}"
    done

    echo -e "\n${BOLD}${CYAN}=== Fontes Supranacionais - Pack Global (6 fontes) ===${RESET}"
    for item in "${GLOBAL_SOURCES[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        printf "  %-26s %s\n" "${id}" "${name}"
    done

    echo -e "\n${BOLD}Total de Fontes Oficiais Cadastradas: 26${RESET}"
}

run_source_test() {
    local test_fn="$1"
    local source_id="$2"
    local source_name="$3"

    printf "  %-28s " "${source_id}"
    local start_ms
    if date +%s%N 2>&1 | grep -qv "N"; then
        start_ms=$(( $(date +%s%N) / 1000000 ))
    else
        start_ms=$(python3 -c "import time; print(int(time.time()*1000))" 2>/dev/null || echo 0)
    fi
    
    # Executa o teste unitário específico no harness Rust
    local output
    if output=$(cargo test -q --test test_each_source "test_source_${test_fn}" -- --exact 2>&1); then
        local end_ms
        if date +%s%N 2>&1 | grep -qv "N"; then
            end_ms=$(( $(date +%s%N) / 1000000 ))
        else
            end_ms=$(python3 -c "import time; print(int(time.time()*1000))" 2>/dev/null || echo 0)
        fi
        local diff_ms=$(( end_ms - start_ms ))
        if [ "${diff_ms}" -le 0 ]; then diff_ms=1; fi
        echo -e "${GREEN}✓ APROVADO${RESET} (${diff_ms}ms) - ${source_name}"
        return 0
    else
        echo -e "${RED}✗ FALHOU${RESET} - ${source_name}"
        echo -e "${RED}${output}${RESET}"
        return 1
    fi
}

test_source_group() {
    local group_name="$1"
    shift
    local -a items=("$@")
    
    echo -e "\n${BOLD}${MAGENTA}▶ Testando Grupo: ${group_name} (${#items[@]} fontes)${RESET}"
    local group_failed=0

    for item in "${items[@]}"; do
        IFS=":" read -r fn id name <<< "${item}"
        if ! run_source_test "${fn}" "${id}" "${name}"; then
            group_failed=$((group_failed + 1))
        fi
    done

    return ${group_failed}
}

# Tratamento de argumentos
if [ $# -eq 0 ]; then
    print_help
    exit 0
fi

TOTAL_FAILED=0

case "$1" in
    -h|--help)
        print_help
        exit 0
        ;;
    -l|--list)
        list_sources
        exit 0
        ;;
    -a|--all)
        echo -e "${BOLD}${CYAN}================================================================${RESET}"
        echo -e "${BOLD}${CYAN}       BRHealth - Teste Exaustivo de Todas as 26 Fontes         ${RESET}"
        echo -e "${BOLD}${CYAN}================================================================${RESET}"
        
        test_source_group "DATASUS" "${DATASUS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
        test_source_group "IBGE" "${IBGE_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
        test_source_group "MDS" "${MDS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
        test_source_group "Ambiental e Clima" "${ENVIRONMENTAL_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
        test_source_group "Global / Supranacional" "${GLOBAL_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
        ;;
    -p|--pack)
        if [ $# -lt 2 ]; then
            echo -e "${RED}Erro: Especifique o pacote (--pack brasil, --pack global, --pack datasus, etc.)${RESET}"
            exit 1
        fi
        PACK_TARGET="$(echo "$2" | tr '[:upper:]' '[:lower:]')"
        case "${PACK_TARGET}" in
            brasil|br|national)
                test_source_group "DATASUS" "${DATASUS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                test_source_group "IBGE" "${IBGE_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                test_source_group "MDS" "${MDS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                test_source_group "Ambiental e Clima" "${ENVIRONMENTAL_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            global|supranational)
                test_source_group "Global / Supranacional" "${GLOBAL_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            datasus)
                test_source_group "DATASUS" "${DATASUS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            ibge)
                test_source_group "IBGE" "${IBGE_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            mds)
                test_source_group "MDS" "${MDS_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            environmental|clima|ambiente)
                test_source_group "Ambiental e Clima" "${ENVIRONMENTAL_SOURCES[@]}" || TOTAL_FAILED=$((TOTAL_FAILED + $?))
                ;;
            *)
                echo -e "${RED}Pacote desconhecido: ${PACK_TARGET}${RESET}"
                echo -e "Pacotes válidos: brasil, global, datasus, ibge, mds, environmental"
                exit 1
                ;;
        esac
        ;;
    *)
        # Teste de fonte individual por id ou substring
        SEARCH_QUERY="$(echo "$1" | tr '[:upper:]' '[:lower:]')"
        ALL_SOURCES=("${DATASUS_SOURCES[@]}" "${IBGE_SOURCES[@]}" "${MDS_SOURCES[@]}" "${ENVIRONMENTAL_SOURCES[@]}" "${GLOBAL_SOURCES[@]}")
        FOUND=0

        echo -e "${BOLD}${CYAN}Buscando testes para a fonte: '${SEARCH_QUERY}'...${RESET}\n"
        for item in "${ALL_SOURCES[@]}"; do
            IFS=":" read -r fn id name <<< "${item}"
            if [[ "${id}" == *"${SEARCH_QUERY}"* ]] || [[ "${fn}" == *"${SEARCH_QUERY}"* ]]; then
                FOUND=$((FOUND + 1))
                if ! run_source_test "${fn}" "${id}" "${name}"; then
                    TOTAL_FAILED=$((TOTAL_FAILED + 1))
                fi
            fi
        done

        if [ ${FOUND} -eq 0 ]; then
            echo -e "${RED}Nenhuma fonte correspondente encontrada para '${SEARCH_QUERY}'.${RESET}"
            echo -e "Use './scripts/test_sources.sh --list' para ver todas as fontes cadastradas."
            exit 1
        fi
        ;;
esac

echo -e "\n${BOLD}${CYAN}================================================================${RESET}"
if [ ${TOTAL_FAILED} -eq 0 ]; then
    echo -e "${BOLD}${GREEN}        ✓ TODAS AS FONTES TESTADAS FORAM APROVADAS!             ${RESET}"
    echo -e "${BOLD}${CYAN}================================================================${RESET}"
    exit 0
else
    echo -e "${BOLD}${RED}        ✗ FALHA NO TESTE DE ${TOTAL_FAILED} FONTE(S)!                    ${RESET}"
    echo -e "${BOLD}${CYAN}================================================================${RESET}"
    exit 1
fi
