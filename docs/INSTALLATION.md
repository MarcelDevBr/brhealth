<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Guia de Instalação e Configuração Modular - BRHealth

O **BRHealth** foi projetado com uma arquitetura estritamente desacoplada. O núcleo do motor analítico (**Core & CLI**) não possui dependências externas além do compilador Rust.

As dependências de linguagens externas (**Python**, **R**, **Java/Kotlin** e **C++**) são **completamente isoladas e opcionais**. Você deve instalar **apenas** o que for utilizar no seu fluxo de trabalho.

---

## 🎯 Escolha sua Trilha de Instalação

O BRHealth tem como foco principal de análise as linguagens **Python** e **R**. Para sistemas integrados e serviços legados, oferece também suporte a **Java 21+/Kotlin** e **C++20**:

```text
                               +-----------------------------+
                               |     BRHealth Core (Rust)    |
                               |    (Apenas Rust 1.85+)      |
                               +--------------+--------------+
                                              |
        +----------------------+--------------+--------------+----------------------+
        |                      |                             |                      |
        v                      v                             v                      v
+---------------+      +---------------+             +---------------+      +---------------+
|    Trilha 1   |      |    Trilha 2   |             |    Trilha 3   |      |    Trilha 4   |
|   🐍 Python   |      |      📊 R     |             |    ☕ Java    |      |    ⚡ C++20   |
| (Data Science)|      | (Bioestatíst.)|             |   & Kotlin    |      |  (Sistemas)   |
+---------------+      +---------------+             +---------------+      +---------------+
| Python 3.10+  |      | R 4.2+        |             | JDK 21+       |      | Clang 15+ ou  |
| Maturin / Pip |      | Pacote arrow  |             | Project Panama|      | GCC 12+       |
+---------------+      +---------------+             +---------------+      +---------------+
```

---

## Índice

1. [Núcleo Base: Instalação do Core e CLI (Rust Puro)](#1-núcleo-base-instalação-do-core-e-cli-rust-puro)
2. [Trilha 1: Python (Ciência de Dados / Polars / PyTorch)](#2-trilha-1-python-ciência-de-dados--polars--pytorch)
3. [Trilha 2: R (Bioestatística e Epidemiologia)](#3-trilha-2-r-bioestatística-e-epidemiologia)
4. [Trilha 3: C++20 (Aplicações de Alta Performance)](#4-trilha-3-c20-aplicações-de-alta-performance)
5. [Trilha 4: Java 21+ e Kotlin (Project Panama FFM)](#5-trilha-4-java-21-e-kotlin-project-panama-ffm)
6. [Instalação de Pré-requisitos do Sistema por SO](#6-instalação-de-pré-requisitos-do-sistema-por-so)
7. [Validação da Instalação](#7-validação-da-instalação)
8. [Solução de Problemas Comuns (Troubleshooting)](#8-solução-de-problemas-comuns-troubleshooting)

---

## 1. Núcleo Base: Instalação do Core e CLI (Rust Puro)

> [!NOTE]
> Se você pretende utilizar o BRHealth apenas via **Linha de Comando (CLI)** ou como **biblioteca Rust**, você **NÃO precisa de Python, R, Java, CMake ou Clang**. Apenas o Rust é necessário.

### Pré-requisitos:
- **Rust Toolchain 1.85+** (Edição 2024).
- **Git**.

### Instalação:
```bash
# 1. Instalar o Rust (caso ainda não possua)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update stable

# 2. Clonar o repositório
git clone https://github.com/MarcelDevBr/brhealth.git
cd brhealth

# 3. Compilar e instalar a CLI globalmente
cargo install --path crates/brhealth-cli
```

### Verificação do CLI:
```bash
brhealth version
brhealth sources
```

---

## 2. Trilha 1: Python (Ciência de Dados / Polars / PyTorch)

Recomendada para cientistas de dados, epidemiologistas computacionais e engenheiros de Machine Learning que utilizam notebooks Jupyter, Polars, Pandas e PyTorch.

### Método A: Instalação via PyPI (Recomendado)

Para instalar a versão oficial estável via PyPI:

```bash
pip install brhealth
```

No **Google Colab** ou em ambientes de **Jupyter Notebook**:
```bash
!pip install brhealth
```

---

### Método B: Instalação Direta via Git (Google Colab / Versão de Desenvolvimento)

Para instalar diretamente a versão mais recente em desenvolvimento a partir do repositório GitHub (sem necessidade de clonar manualmente):

```bash
pip install git+https://github.com/MarcelDevBr/brhealth.git#subdirectory=crates/brhealth-python
```

No **Google Colab** ou em ambientes de **Jupyter Notebook**:
```bash
!pip install git+https://github.com/MarcelDevBr/brhealth.git#subdirectory=crates/brhealth-python
```

---

### Método C: Compilação Local com Maturin (Desenvolvimento e Contribuição)

Recomendado para contribuidores que desejam modificar o código em Rust ou na extensão PyO3.

#### Pré-requisitos Adicionais:
- **Python 3.10+** (Recomendado: 3.11 ou 3.12).
- **Rust Toolchain 1.85+** (Edição 2024).
- **Maturin** (compilador de extensões nativas PyO3).

#### Passo a Passo de Instalação:

```bash
# 1. A partir da raiz do repositório brhealth
cd brhealth

# 2. Criar e ativar um ambiente virtual isolado
python3 -m venv .venv
source .venv/bin/activate  # No Windows: .venv\Scripts\Activate.ps1

# 3. Instalar o Maturin e as bibliotecas analíticas
pip install --upgrade pip
pip install maturin polars pyarrow torch

# 4. Compilar e instalar o brhealth no ambiente virtual Python
cd crates/brhealth-python
maturin develop --release
cd ../..
```

### Verificação da Instalação:
```bash
python3 -c "import brhealth; print('Versão BRHealth:', brhealth.__version__); print('DV SP:', brhealth.calculate_ibge_dv('355030'))"
```

---

## 3. Trilha 2: R (Bioestatística e Epidemiologia)

O **R** é a linguagem canônica da bioestatística e saúde pública no Brasil. A integração com o BRHealth opera através de interoperabilidade de memória contígua em **Apache Arrow** ou via **Reticulate**.

### Pré-requisitos Adicionais:
- **R 4.2+**
- Pacotes R: `arrow`, `reticulate` e `dplyr` (opcional: `tidyverse`).

### Método A: Integração R com Apache Arrow via Reticulate (Zero-Copy)

Este método permite chamar os métodos do BRHealth diretamente no R e converter os resultados para tabelas Arrow nativas do R sem nenhuma cópia de dados em disco ou rede:

```bash
# 1. No terminal, certifique-se de que o pacote Python foi compilado (Trilha 1)
# 2. Abra o console do R e instale as dependências:
```

```r
install.packages(c("reticulate", "arrow", "dplyr"))
```

Configuração no script R:
```r
library(reticulate)
library(arrow)
library(dplyr)

# Apontar para o ambiente virtual onde o brhealth foi instalado
use_virtualenv("./.venv", required = TRUE)

brhealth <- import("brhealth")

# Testar chamada de função
print(brhealth$calculate_ibge_dv("355030")) # Retorna 8
```

### Método B: Pipeline Colunar CLI -> Arquivo Parquet -> R

Para análises massivas de bases estaduais e nacionais, utilize a CLI para extrair microdados brutos em Parquet particionado e carregue instantaneamente em R:

```bash
# Executa extração colunar com harmonização e persistência no cache da HOME (~/.brhealth)
brhealth fetch --source datasus.sih --uf SP --year 2023 --month 5 --enrich-csap --out-parquet ~/.brhealth/data/sih_sp.parquet
```

No R:
```r
library(arrow)
library(dplyr)

# Leitura multithreaded de alta performance
dados_sih <- arrow::read_parquet("~/.brhealth/data/sih_sp.parquet")

# Análise de internações evitáveis na Atenção Primária
dados_csap <- dados_sih %>%
  filter(is_csap == TRUE) %>%
  group_by(municipio_residencia) %>%
  summarise(
    total_internacoes = n(),
    custo_total = sum(valor_total_pago, na.rm = TRUE)
  )
```

---

## 4. Trilha 3: C++20 (Aplicações de Alta Performance)

Recomendada para engenheiros de sistemas que precisam embutir o BRHealth em softwares legados ou pipelines de infraestrutura em C++.

### Pré-requisitos Adicionais:
- **Clang 15+** ou **GCC 12+** (com suporte total a C++20).
- **CMake 3.22+**.

### Passo a Passo:

```bash
# 1. Compilar a biblioteca C-ABI compartilhada
cargo build --release -p brhealth-ffi

# 2. O header C++20 com gerenciamento RAII está em:
# bindings/cpp/include/brhealth.hpp

# 3. A biblioteca dinâmica gerada está em:
# target/release/libbrhealth_ffi.so (Linux) ou .dylib (macOS) / .dll (Windows)
```

Exemplo de compilação de binário C++20:
```bash
g++ -std=c++20 seu_programa.cpp \
    -Ibindings/cpp/include \
    -Ltarget/release \
    -lbrhealth_ffi \
    -Wl,-rpath,target/release \
    -o seu_programa
```

---

## 5. Trilha 4: Java 21+ e Kotlin (Project Panama FFM)

Recomendada para microsserviços empresariais em Java/Kotlin (Spring Boot, Quarkus, Micronaut). Utiliza a moderna **Foreign Function & Memory API (FFM)** do Java 21, eliminando o JNI tradicional e acessando ponteiros nativos sem custo de cópia.

### Pré-requisitos Adicionais:
- **JDK 21+** (OpenJDK, Temurin ou Oracle).

### Passo a Passo:

```bash
# 1. Compilar a biblioteca dinâmica para JVM
cargo build --release -p brhealth-jni

# 2. A classe de interface Java 21 está em:
# bindings/jvm/BRHealthEngine.java

# 3. Compilar e executar com flags de preview nativo do Java 21:
javac --release 21 bindings/jvm/BRHealthEngine.java
```

Execução em Java:
```bash
java --enable-native-access=ALL-UNNAMED -cp bindings/jvm SeuAppJava
```

---

## 6. Instalação de Pré-requisitos do Sistema por SO

### Ubuntu / Debian (22.04 LTS ou 24.04 LTS)
```bash
# Apenas para o Core:
sudo apt update && sudo apt install -y curl git build-essential

# Opcional (se for usar Python):
sudo apt install -y python3 python3-pip python3-venv python3-dev

# Opcional (se for usar R):
sudo apt install -y r-base r-base-dev

# Opcional (se for usar Java 21):
sudo apt install -y openjdk-21-jdk

# Opcional (se for usar C++20):
sudo apt install -y clang llvm libclang-dev cmake
```

### Fedora (39+)
```bash
# Core:
sudo dnf groupinstall -y "Development Tools" && sudo dnf install -y git

# Trilha Python: sudo dnf install -y python3-devel
# Trilha R:      sudo dnf install -y R R-devel
# Trilha Java:   sudo dnf install -y java-21-openjdk-devel
# Trilha C++:    sudo dnf install -y clang clang-devel cmake
```

### macOS (Apple Silicon e Intel via Homebrew)
```bash
# Core:
xcode-select --install

# Trilha Python: brew install python@3.12
# Trilha R:      brew install r
# Trilha Java:   brew install openjdk@21
# Trilha C++:    brew install llvm cmake
```

### Windows (WSL2 ou Nativo)
- **Recomendado**: Instale o **WSL2 com Ubuntu 24.04 LTS** e siga a instalação do Ubuntu.
- **Nativo**: Instale o Rust via `rustup.rs`, Visual Studio 2022 Build Tools (MSVC) e Python/R separadamente conforme sua necessidade.

---

## 7. Validação da Instalação

Independentemente das trilhas que você configurou, execute a suíte de testes automatizados do workspace para garantir integridade matemática e de memória:

```bash
# 1. Executar testes de unidade e integração
cargo test --workspace

# 2. Executar testes de documentação técnica
cargo test --workspace --doc

# 3. Validação de conformidade estrita de código (Zero Warnings)
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 8. Solução de Problemas Comuns (Troubleshooting)

### A. "error: package require rustc 1.85.0 or newer"
- **Causa**: Sua versão do compilador Rust está abaixo da Edição 2024.
- **Solução**:
  ```bash
  rustup update stable
  rustc --version
  ```

### B. "Maturin failed to find Python development headers"
- **Causa**: O pacote de desenvolvimento do Python não está instalado no sistema.
- **Solução (Linux)**: `sudo apt install -y python3-dev` (Debian/Ubuntu) ou `sudo dnf install -y python3-devel` (Fedora).

### C. "package 'arrow' is not available for this version of R"
- **Causa**: Repositório CRAN padrão sem binários atualizados do Apache Arrow.
- **Solução**: No console do R, utilize o instalador com suporte a Arrow C++:
  ```r
  install.packages("arrow", repos = c("https://apache.r-universe.dev", "https://cloud.r-project.org"))
  ```

### D. "UnsatisfiedLinkError no Java ao carregar libbrhealth_jni"
- **Causa**: O caminho da biblioteca compartilhada (`.so`/`.dylib`/`.dll`) não foi encontrado.
- **Solução**: Passe o caminho absoluto ao instanciar `BRHealthEngine(libPath)` ou adicione o diretório `target/release` à variável de ambiente `LD_LIBRARY_PATH` (Linux) ou `DYLD_LIBRARY_PATH` (macOS).
