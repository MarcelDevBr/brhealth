# Scripts Operacionais e de Automação Multiplataforma - BRHealth

Este diretório reúne os scripts operacionais e executáveis para verificação contínua da qualidade analítica, testes de fontes de dados e manutenção de caches do **BRHealth**, projetados para funcionar de forma **100% agnóstica de sistema operacional (Linux, macOS e Windows)**.

---

## Modos de Execução por Sistema Operacional

O BRHealth oferece três formas equivalentes e independentes de executar os testes e tarefas de manutenção:

| Plataforma | Modo Recomendado | Alternativa Shell Nativa |
|---|---|---|
| **Universal (Qualquer OS)** | `python3 scripts/<script>.py` | Binário CLI `brhealth` / `cargo` |
| **Linux / macOS** | `./scripts/<script>.sh` | `python3 scripts/<script>.py` |
| **Windows (PowerShell)** | `.\scripts\<script>.ps1` | `python scripts\<script>.py` |
| **Windows (CMD)** | `python scripts\<script>.py` | `cargo run -p brhealth-cli -- ...` |

---

## 1. Execução Completa da Suíte de Testes

Executa o pipeline completo de validação do workspace: compilação estática, clippy rigoroso com `-D warnings`, testes unitários, testes de integração dos country packs, bindings Python (PyCapsule / DLPack), doc-tests matemáticos, testes granulares das 26 fontes e testes da CLI binária.

```bash
# Universal (Linux, macOS, Windows)
python3 scripts/run_all_tests.py

# Linux / macOS (Bash)
./scripts/run_all_tests.sh

# Windows (PowerShell)
.\scripts\run_all_tests.ps1
```

---

## 2. Limpeza de Caches e Temporários

Remove com segurança e de forma idempotente os caches analíticos temporários, particionamentos transitórios Hive-Parquet, artefatos Python e bancos de dados SQLite transitórios. Detecta automaticamente os caminhos de temp específicos do OS (`%LOCALAPPDATA%\Temp` no Windows, `/var/folders` no macOS e `/tmp` no Linux).

```bash
# 1. Modo Universal via Python (Linux, macOS, Windows)
python3 scripts/clean_cache.py            # Preserva a pasta target/
python3 scripts/clean_cache.py --cargo    # Limpa também com 'cargo clean'

# 2. Modo Nativo Rust via CLI BRHealth (Zero dependências externas)
cargo run -p brhealth-cli -- clean-cache
cargo run -p brhealth-cli -- clean-cache --cargo

# 3. Linux / macOS (Bash)
./scripts/clean_cache.sh
./scripts/clean_cache.sh --cargo

# 4. Windows (PowerShell)
.\scripts\clean_cache.ps1
.\scripts\clean_cache.ps1 -Cargo
```

---

## 3. Testes Granulares por Fonte de Dados (26 Fontes Oficiais)

Permite testar de forma isolada, por grupo ou exaustiva cada uma das **26 fontes oficiais** suportadas pelo motor analítico BRHealth (DATASUS, IBGE, MDS, órgãos ambientais e fontes globais/supranacionais).

```bash
# Universal (Linux, macOS, Windows)
python3 scripts/test_sources.py --list               # Listar todas as 26 fontes
python3 scripts/test_sources.py --all                # Testar todas as 26 fontes
python3 scripts/test_sources.py --pack brasil        # Testar 20 fontes nacionais
python3 scripts/test_sources.py --pack global        # Testar 6 fontes globais
python3 scripts/test_sources.py datasus.sim          # Testar fonte individual SIM
python3 scripts/test_sources.py ibge.censo           # Testar fonte individual Censo
python3 scripts/test_sources.py copernicus_era5      # Testar fonte individual ERA5

# Linux / macOS (Bash)
./scripts/test_sources.sh --all
./scripts/test_sources.sh --pack datasus
./scripts/test_sources.sh datasus.sim

# Windows (PowerShell)
.\scripts\test_sources.ps1 -All
.\scripts\test_sources.ps1 -Pack ibge
.\scripts\test_sources.ps1 -Source datasus.sim

# Nativamente via Cargo (Qualquer OS)
cargo test --test test_each_source test_source_datasus_sim
cargo test --test test_each_source test_source_ibge_censo
cargo test --test test_each_source test_source_global_copernicus_era5
```

---

## Licença e Direitos Autorais

Todos os scripts são licenciados sob a **GNU Affero General Public License v3 (AGPLv3)**.  
*Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.*
