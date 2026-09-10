<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Guia de Instalação e Compilação - BRHealth

Este documento fornece as instruções completas, passo a passo, para configurar o ambiente de desenvolvimento, compilar e instalar todos os componentes do **BRHealth** em múltiplos sistemas operacionais (Linux, macOS e Windows).

---

## Índice

1. [Requisitos de Sistema e Ferramentas](#1-requisitos-de-sistema-e-ferramentas)
2. [Instalação de Pré-requisitos por Sistema Operacional](#2-instalação-de-pré-requisitos-por-sistema-operacional)
   - [Linux (Ubuntu / Debian / Fedora / Arch)](#linux-ubuntu--debian--fedora--arch)
   - [macOS (Apple Silicon e Intel)](#macos-apple-silicon-e-intel)
   - [Windows (WSL2 e Nativo com MSVC)](#windows-wsl2-e-nativo-com-msvc)
3. [Clonagem do Repositório](#3-clonagem-do-repositório)
4. [Compilação e Instalação do Motor Rust (brhealth-core e brhealth-cli)](#4-compilação-e-instalação-do-motor-rust)
5. [Instalação dos Bindings Python (brhealth-python)](#5-instalação-dos-bindings-python)
6. [Compilação da C-ABI e Bindings C++20 (brhealth-ffi)](#6-compilação-da-c-abi-e-bindings-c20)
7. [Configuração dos Bindings Java 21+ Panama FFM (brhealth-jni)](#7-configuração-dos-bindings-java-21-panama-ffm)
8. [Validação e Verificação da Instalação](#8-validação-e-verificação-da-instalação)
9. [Solução de Problemas Comuns (Troubleshooting)](#9-solução-de-problemas-comuns-troubleshooting)

---

## 1. Requisitos de Sistema e Ferramentas

| Componente | Versão Mínima | Finalidade |
| :--- | :--- | :--- |
| **Rust Toolchain** | **1.85+** (Rust Edição 2024) | Compilação do núcleo `brhealth-core`, CLI e FFI |
| **Cargo** | Incluído no Rust | Gerenciador de pacotes e compilação do workspace |
| **Clang / LLVM** | **15.0+** | Necessário para bindings FFI e compilação C++20 |
| **CMake** | **3.22+** | Utilizado para compilar exemplos e bibliotecas C++ |
| **Python** | **3.10+** (Recomendado: 3.11 ou 3.12) | Uso dos bindings Python e ecossistema de dados |
| **Maturin** | **1.4+** | Compilação e empacotamento dos bindings PyO3 em rodas nativas |
| **JDK (Java)** | **21+** (OpenJDK ou Oracle) | Necessário apenas para o `brhealth-jni` (Project Panama FFM) |
| **Git** | **2.30+** | Controle de versão e download do código-fonte |

> [!IMPORTANT]
> O BRHealth utiliza a **Edição 2024 do Rust** (`edition = "2024"`). Certifique-se de que sua versão do `rustc` seja **1.85.0 ou superior**. Atualize via `rustup update stable`.

---

## 2. Instalação de Pré-requisitos por Sistema Operacional

### Linux (Ubuntu / Debian / Fedora / Arch)

#### Ubuntu / Debian (22.04 LTS ou 24.04 LTS):
```bash
# Atualizar repositórios do sistema
sudo apt update && sudo apt upgrade -y

# Instalar ferramentas de compilação essenciais, Clang e Python
sudo apt install -y build-essential curl git clang llvm libclang-dev cmake \
    python3 python3-pip python3-venv python3-dev pkg-config

# Instalar o Rust via rustup (caso ainda não possua)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update stable

# (Opcional) Instalar OpenJDK 21 para Java Panama
sudo apt install -y openjdk-21-jdk
```

#### Fedora (39+):
```bash
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y clang clang-devel llvm llvm-devel cmake python3-devel java-21-openjdk-devel
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

#### Arch Linux:
```bash
sudo pacman -Syu --needed base-devel clang llvm cmake python python-pip rustup jdk21-openjdk
rustup default stable
```

---

### macOS (Apple Silicon e Intel)

No macOS, utilize o [Homebrew](https://brew.sh/):

```bash
# 1. Instalar Command Line Tools do Xcode
xcode-select --install

# 2. Instalar dependências via Homebrew
brew install cmake llvm python@3.12 openjdk@21

# 3. Instalar ou atualizar o Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update stable

# 4. Configurar variáveis para LLVM (caso necessário no shell ~/.zshrc)
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm/include"
```

---

### Windows (WSL2 e Nativo com MSVC)

#### Opção A: WSL2 (Altamente Recomendada)
A experiência recomendada no Windows é utilizar o **WSL2 com Ubuntu 24.04 LTS**. Siga as mesmas instruções da seção [Linux (Ubuntu / Debian)](#linux-ubuntu--debian--fedora--arch).

#### Opção B: Windows Nativo com MSVC
1. Instale o **Visual Studio Community 2022** com a carga de trabalho *"Desenvolvimento para Desktop com C++"*.
2. Instale o instalador do Rust a partir de [rustup.rs](https://rustup.rs/) (selecione a toolchain `x86_64-pc-windows-msvc`).
3. Instale o Python 3.11+ via instalador oficial ou `winget install Python.Python.3.12`.
4. Instale o LLVM nativo via `winget install LLVM.LLVM`.
5. No terminal PowerShell (como Administrador), defina a variável `LIBCLANG_PATH`:
   ```powershell
   [System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", [System.EnvironmentVariableTarget]::User)
   ```

---

## 3. Clonagem do Repositório

```bash
git clone https://github.com/MarcelDevBr/brhealth.git
cd brhealth
```

Verifique o status do workspace Cargo:
```bash
cargo check --workspace
```

---

## 4. Compilação e Instalação do Motor Rust

### Compilar Todo o Workspace em Modo Release

O BRHealth utiliza otimizações pesadas de compilação (*Link-Time Optimization* `lto = "fat"`, `opt-level = 3` e remoção de símbolos de depuração para máxima performance):

```bash
cargo build --release --workspace
```

Os binários e bibliotecas compartilhadas geradas estarão localizados em:
- CLI: `target/release/brhealth`
- FFI C-ABI: `target/release/libbrhealth_ffi.so` (Linux), `.dylib` (macOS), `.dll` (Windows)
- JNI/Panama: `target/release/libbrhealth_jni.so` (Linux), `.dylib` (macOS), `.dll` (Windows)

### Instalar o CLI Globalmente no Sistema

Para disponibilizar o utilitário `brhealth` diretamente no terminal (`$PATH`):

```bash
cargo install --path crates/brhealth-cli
```

Teste a instalação do CLI:
```bash
brhealth version
brhealth sources
```

---

## 5. Instalação dos Bindings Python

Os bindings do BRHealth utilizam **PyO3** e são compatíveis com o ecossistema colunar Python (**Polars**, **PyArrow**, **Pandas**, **DuckDB** e **PyTorch** via DLPack).

### Passo 1: Criar e Ativar Ambiente Virtual

```bash
python3 -m venv .venv
source .venv/bin/activate  # No Windows: .venv\Scripts\Activate.ps1
pip install --upgrade pip
```

### Passo 2: Instalar Maturin e Dependências de Análise

```bash
pip install maturin polars pyarrow torch
```

### Passo 3: Compilar e Instalar o Pacote em Modo Desenvolvimento

Navegue até a pasta da crate Python ou instale na raiz via maturin:

```bash
cd crates/brhealth-python
maturin develop --release
cd ../..
```

### Passo 4: Verificar a Instalação no Python

```bash
python3 -c "import brhealth; print('BRHealth Python Version:', brhealth.__version__); print('DV SP:', brhealth.calculate_ibge_dv('355030'))"
```

Saída esperada:
```text
BRHealth Python Version: 0.1.0
DV SP: 8
```

---

## 6. Compilação da C-ABI e Bindings C++20

A crate `crates/brhealth-ffi` exporta uma C-ABI plana com a especificação **Apache Arrow C Data Interface** (`ArrowSchema` e `ArrowArray`).

O cabeçalho C++20 idiomático com gerenciamento seguro de memória RAII está localizado em `bindings/cpp/include/brhealth.hpp`.

### Compilar a Biblioteca Dinâmica:
```bash
cargo build --release -p brhealth-ffi
```

### Exemplo de Compilação de um Programa C++20:

Crie um arquivo de teste `main.cpp`:
```cpp
#include "bindings/cpp/include/brhealth.hpp"
#include <iostream>

int main() {
    std::cout << "BRHealth Version: " << brhealth::version() << std::endl;
    uint8_t dv = brhealth::calculate_ibge_dv("355030");
    std::cout << "DV São Paulo: " << static_cast<int>(dv) << std::endl;
    return 0;
}
```

Compile com GCC 12+ ou Clang 15+:
```bash
g++ -std=c++20 main.cpp -I. -Ltarget/release -lbrhealth_ffi -Wl,-rpath,target/release -o test_cpp
./test_cpp
```

---

## 7. Configuração dos Bindings Java 21+ Panama FFM

O BRHealth oferece suporte a Java 21+ através da moderna **Foreign Function & Memory API (Project Panama)**, eliminando o JNI tradicional e garantindo interoperabilidade com zero cópia de memória.

### Compilar a Biblioteca C-ABI/JNI:
```bash
cargo build --release -p brhealth-jni
```

### Compilar e Executar com Java 21:
```bash
javac --enable-preview --release 21 bindings/jvm/BRHealthEngine.java
```

Para executar um teste em Java, passe a flag `--enable-native-access=ALL-UNNAMED`:
```bash
java --enable-preview --enable-native-access=ALL-UNNAMED -cp bindings/jvm MeuPrograma
```

---

## 8. Validação e Verificação da Instalação

Após compilar o projeto, execute a suíte completa de testes para garantir que todos os 100+ testes unitários, testes de integração e verificações de robustez sejam aprovados:

```bash
# 1. Executar todos os testes automatizados do workspace
cargo test --workspace

# 2. Executar testes de documentação formal (Doc-tests)
cargo test --workspace --doc

# 3. Validar conformidade de código estrita (Zero Warnings no Clippy)
cargo clippy --workspace --all-targets -- -D warnings

# 4. (Opcional) Executar benchmarks científicos com Criterion
cargo bench -p brhealth-core
```

---

## 9. Solução de Problemas Comuns (Troubleshooting)

### Erro: `error: package require rustc 1.85.0 or newer`
- **Causa**: Sua toolchain do Rust está desatualizada para a Edição 2024.
- **Solução**:
  ```bash
  rustup update stable
  rustc --version
  ```

### Erro: `libclang.so: cannot open shared object file` durante `cargo build`
- **Causa**: O compilador Clang / LLVM não foi encontrado no sistema ou não está no `$PATH`.
- **Solução (Linux)**:
  ```bash
  sudo apt install -y libclang-dev clang
  export LIBCLANG_PATH=/usr/lib/llvm-18/lib  # Ajuste conforme a versão instalada
  ```

### Erro: `cargo clippy` reporta avisos em branches de teste
- **Causa**: O BRHealth adota tolerância zero a alertas de compilação.
- **Solução**: Execute `cargo clippy --fix --workspace` ou verifique os detalhes apontados pelo linter. Todo código deve compilar sob `cargo clippy --workspace --all-targets -- -D warnings`.

### Erro: Falha de conexão FTP ao executar `brhealth fetch`
- **Causa**: Os servidores de FTP do DATASUS (`ftp.datasus.gov.br`) frequentemente bloqueiam conexões ativas ou estão sob manutenção.
- **Solução**: O BRHealth utiliza conexão passiva assíncrona com retentativas automáticas e backoff exponencial. Caso o servidor esteja fora do ar, aguarde alguns minutos ou utilize dados em cache local.
