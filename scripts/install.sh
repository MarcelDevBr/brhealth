#!/usr/bin/env bash
# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

set -euo pipefail

REPO="MarcelDevBr/brhealth"
GITHUB_URL="https://github.com/${REPO}"

echo "========================================================"
echo "    BRHealth CLI - Instalador de Binário Pré-compilado"
echo "========================================================"

# 1. Identificar Sistema Operacional
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM="linux";;
    Darwin*)    PLATFORM="darwin";;
    *)
        echo "ERRO: Sistema operacional não suportado: ${OS}" >&2
        exit 1
        ;;
esac

# 2. Identificar Arquitetura
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)   CPU="x86_64";;
    aarch64|arm64)  CPU="aarch64";;
    *)
        echo "ERRO: Arquitetura de CPU não suportada: ${ARCH}" >&2
        exit 1
        ;;
esac

# 3. Mapear para Target Triple
if [ "${PLATFORM}" = "linux" ]; then
    if [ "${CPU}" = "x86_64" ]; then
        TARGET="x86_64-unknown-linux-musl"
    else
        TARGET="aarch64-unknown-linux-gnu"
    fi
elif [ "${PLATFORM}" = "darwin" ]; then
    if [ "${CPU}" = "aarch64" ]; then
        TARGET="aarch64-apple-darwin"
    else
        TARGET="x86_64-apple-darwin"
    fi
fi

echo "-> Plataforma detectada: ${PLATFORM} (${CPU})"
echo "-> Target selecionado:   ${TARGET}"

# 4. Determinar Versão
VERSION="${BRHEALTH_VERSION:-}"
if [ -z "${VERSION}" ]; then
    echo "-> Buscando a versão estável mais recente..."
    LATEST_RELEASE_JSON="$(curl -sSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)"
    VERSION="$(echo "${LATEST_RELEASE_JSON}" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d '"' -f 4 || true)"
    
    if [ -z "${VERSION}" ]; then
        VERSION="v1.0.0"
        echo "-> Não foi possível consultar a API do GitHub (rate limit?). Usando versão padrão: ${VERSION}"
    else
        echo "-> Versão mais recente encontrada: ${VERSION}"
    fi
else
    echo "-> Usando versão especificada: ${VERSION}"
fi

# 5. URLs de Download
ARCHIVE_NAME="brhealth-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="${GITHUB_URL}/releases/download/${VERSION}/${ARCHIVE_NAME}"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "-> Baixando ${ARCHIVE_NAME}..."
if ! curl -fSL --progress-bar "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}"; then
    echo "ERRO: Falha ao baixar o binário de ${DOWNLOAD_URL}" >&2
    exit 1
fi

# 6. Verificação Criptográfica SHA-256 (FAIR / Proveniência)
echo "-> Verificando integridade SHA-256..."
if curl -fsSL "${CHECKSUM_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}.sha256" 2>/dev/null; then
    cd "${TMP_DIR}"
    if command -v sha256sum &>/dev/null; then
        sha256sum -c "${ARCHIVE_NAME}.sha256" || { echo "ERRO: Checksum SHA-256 inválido!" >&2; exit 1; }
    elif command -v shasum &>/dev/null; then
        shasum -a 256 -c "${ARCHIVE_NAME}.sha256" || { echo "ERRO: Checksum SHA-256 inválido!" >&2; exit 1; }
    else
        echo "AVISO: Utilitário de checagem SHA-256 não disponível. Pulando validação estrita."
    fi
    cd - >/dev/null
else
    echo "AVISO: Arquivo .sha256 não encontrado na release. Continuando..."
fi

# 7. Descompactar
echo "-> Extraindo binário..."
tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "${TMP_DIR}"

if [ ! -f "${TMP_DIR}/brhealth" ]; then
    echo "ERRO: Binário 'brhealth' não encontrado no pacote extraído." >&2
    exit 1
fi

chmod +x "${TMP_DIR}/brhealth"

# 8. Diretório de Destino da Instalação
if [ -n "${BRHEALTH_INSTALL_DIR:-}" ]; then
    INSTALL_DIR="${BRHEALTH_INSTALL_DIR}"
elif [ -d "${HOME}/.local/bin" ] || mkdir -p "${HOME}/.local/bin" 2>/dev/null; then
    INSTALL_DIR="${HOME}/.local/bin"
elif [ -d "${HOME}/.cargo/bin" ]; then
    INSTALL_DIR="${HOME}/.cargo/bin"
else
    INSTALL_DIR="/usr/local/bin"
fi

mkdir -p "${INSTALL_DIR}" 2>/dev/null || true

echo "-> Instalando em ${INSTALL_DIR}/brhealth..."
if [ -w "${INSTALL_DIR}" ]; then
    mv "${TMP_DIR}/brhealth" "${INSTALL_DIR}/brhealth"
else
    echo "-> Requer permissões de administrador para gravar em ${INSTALL_DIR}..."
    sudo mv "${TMP_DIR}/brhealth" "${INSTALL_DIR}/brhealth"
fi

echo ""
echo "========================================================"
echo "    BRHealth CLI instalado com sucesso!"
echo "========================================================"
echo ""
echo "Localização: ${INSTALL_DIR}/brhealth"

# Checar se INSTALL_DIR está no PATH
case ":${PATH}:" in
    *:"${INSTALL_DIR}":*) ;;
    *)
        echo "AVISO: O diretório '${INSTALL_DIR}' não está no seu PATH atual."
        echo "Para executar 'brhealth' de qualquer lugar, adicione a seguinte linha ao seu ~/.bashrc ou ~/.zshrc:"
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        ;;
esac

echo "Para testar, execute:"
echo "    brhealth --help"
echo "    brhealth dv 355030"
echo ""
