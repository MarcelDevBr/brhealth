<div align="center">

# BRHealth

**Motor Analítico Colunar de Alta Performance para Saúde Coletiva e Determinantes Sociais**

[![CI/CD](https://github.com/MarcelDevBr/brhealth/actions/workflows/ci.yml/badge.svg)](https://github.com/MarcelDevBr/brhealth/actions)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_3.0--or--later-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust: 2024 Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![Apache Arrow](https://img.shields.io/badge/Apache_Arrow-53.0-brightgreen.svg)](https://arrow.apache.org/)
[![W3C PROV-O](https://img.shields.io/badge/FAIR-W3C_PROV--O-blueviolet.svg)](https://www.w3.org/TR/prov-o/)
[![Clippy: Zero Warnings](https://img.shields.io/badge/Clippy-Strict_Zero_Warnings-success.svg)](https://github.com/rust-lang/rust-clippy)

</div>

---

## 📌 Navegação Rápida na Documentação

| Guia | Público-Alvo | Descrição |
| :--- | :--- | :--- |
| 📖 **[Guia de Instalação Modular](docs/INSTALLATION.md)** | Pesquisadores, Devs | Instalação desacoplada: Core (Rust puro), Python (Polars/PyTorch), R (Tidyverse/Arrow), C++20 e Java 21+. |
| 🚀 **[Manual de Utilização](docs/USAGE_GUIDE.md)** | Bioestatísticos, Gestores | Manual prático completo: CLI, Python, R (Arrow Zero-Copy), Rust e formulações científicas em LaTeX. |
| 🧩 **[Guia de Extensão e SPI](docs/EXTENDING_BRHEALTH.md)** | Engenheiros, Arquitetos | Como criar novos adaptadores `HealthDataSourceSPI`, Country Packs e decodificadores. |
| 📐 **[Documento de Design de Software (SDD)](docs/architecture/sdd_brhealth.md)** | Arquitetos de Sistemas | Modelo C4, especificação matemática, layout de memória contígua e W3C PROV-O. |

---

## Visão Geral

O **BRHealth** é um motor analítico colunar de alta performance desenvolvido em Rust para ingestão, harmonização, análise bioestatística e modelagem econômica de dados do Sistema Único de Saúde (SUS) e determinantes sociais brasileiros.

O projeto é **modular por design**: o núcleo colunar em Rust opera de forma puramente independente, oferecendo interfaces de primeira classe para **Python** e **R** (para epidemiologia e ciência de dados), além de bindings nativos para **C++20** e **Java 21+/Kotlin (Project Panama FFM)** para microsserviços e sistemas hospitalares.

---

## Arquitetura: Hexagonal Data-Oriented Design (Hexagonal DOD)

```text
               +-------------------------------------------------------------+
               |                       INBOUND PORTS                         |
               |     (CLI / Python PyO3 / Arrow C-API / Java 21+ Panama)     |
               +------------------------------+------------------------------+
                                              |
                                              v
+------------------------------------------------------------------------------------------+
|                                     DOMAIN CORE                                          |
|                       (Puro - Zero I/O, Zero Unwraps, 64-bit Aligned)                    |
|                                                                                          |
|   +-----------------------+   +-----------------------+   +--------------------------+   |
|   |   Canonical Schemas   |   |   Spatial Indexing    |   |     IBGE Harmonizer      |   |
|   |     (Apache Arrow)    |   |    (Uber H3 & S2)     |   |   (Luhn Modulo 10 DV)    |   |
|   +-----------------------+   +-----------------------+   +--------------------------+   |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   |                       Health Economics & CSAP Analytics                          |   |
|   |              (Portaria MS/SAS nº 221/2008 - 19 Grupos de Causas,                 |   |
|   |                  Custos Evitáveis, Dias Evitáveis e ROI da APS)                  |   |
|   +----------------------------------------------------------------------------------+   |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   |                         FAIR Cryptographic Lineage                               |   |
|   |                (W3C PROV-O JSON-LD, SHA-256 On-the-Fly Hashing)                  |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
                                              |
                                              v
               +-------------------------------------------------------------+
               |                       OUTBOUND PORTS                        |
               |          HealthDataSourceSPI / Transport / Decompressor     |
               +------------------------------+------------------------------+
                                              |
                                              v
+------------------------------------------------------------------------------------------+
|                               ADAPTERS & INFRASTRUCTURE                                  |
|                                                                                          |
|   +-------------------------+   +------------------------+   +-----------------------+   |
|   |  Native Blast Decoder   |   |  Country Pack Brasil   |   |  Hive-Parquet Storage |   |
|   |    (PKWARE DCL .dbc)    |   |  (SIM, SINASC, SIH)    |   |     (Time-Travel)     |   |
|   +-------------------------+   +------------------------+   +-----------------------+   |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   |                       Zero-Copy Arrow C Data Interface                           |   |
|   |              (Ponteiros FFI diretos para Python / C++20 / Java 21+)              |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
```

---

## Principais Capacidades

### 1. Descompressor DATASUS Blast PKWARE DCL 100% Nativo
- Decodificação limpa em Rust seguro sem binários externos (`dbc2dbf`) ou bibliotecas C legadas.
- Pipeline colunar DBF $\to$ Apache Arrow `RecordBatch` com 4.000+ linhas por centésimo de segundo.

### 2. Harmonização Territorial IBGE (1970–2026)
- Validação e cálculo canônico do Dígito Verificador (DV) via **Luhn Módulo 10**.
- Tabela imutável de transições históricas municipais (desmembramentos e fusões).

### 3. Indexação Espacial Discreta Uber H3 e Google S2 Geometry
- Resoluções configuráveis (ex: Resolução H3 8 para células de $\sim 0.7\text{ km}^2$; Nível S2 10 para células municipais).
- Funções vetorizadas sobre colunas de coordenadas (`append_h3_column`, `append_s2_column`).
- Análise de vizinhança em anel concêntrico (`grid_disk`) e *spatial joins* colunares contíguos em Arrow (`spatial_join_on_index`).

### 4. Catálogo Unificado de 26 Fontes Oficiais de Dados
- **Country Pack Brasil (`pack_brasil`)**:
  - **DATASUS / MS**: SIM (Mortalidade), SINASC (Nascidos Vivos), SIH (Internações), SINAN (Agravos), SIASUS (Ambulatorial), CNES (Leitos e Unidades), SI-PNI (Vacinas), SISVAN (Nutrição), SISCAN (Câncer), BPS (Preços de Medicamentos).
  - **IBGE & MDS**: Censo Demográfico, PNAD Contínua, POF (Orçamentos Familiares), PeNSE (Saúde Escolar), MUNIC (Gestão Municipal), CadÚnico (Vulnerabilidade Social).
  - **Clima, Ambiente e Saneamento**: INMET (Meteorologia), BDQueimadas / INPE (Focos de Calor), PRODES / INPE (Desmatamento), SISAGUA / SNIS (Qualidade da Água).
- **Country Pack Global (`pack_global`)**:
  - **WHO GHO** (Indicadores Globais da OMS / ODS 3)
  - **IHME GBD** (Carga Global de Doenças - DALYs, YLLs, YLDs)
  - **Copernicus ERA5** (Reanálise Climática Planetária em Grade)
  - **WorldPop** (População em Grade Georreferenciada de 100m)
  - **PAHO / OPAS PLISA** (Vigilância de Arboviroses das Américas)
  - **OpenAQ** (Monitoramento de Qualidade do Ar e Poluentes)

### 5. Bioestatística, Epidemiologia e Mortalidade Prematura (APVP / YLL)
- Cálculo formal dos **Anos Potenciais de Vida Perdidos** (APVP / *Years of Life Lost* - YLL):
  $$\text{APVP} = \sum_{i=1}^{n} d_i \cdot (L - a_i)$$
- Taxa padronizada de APVP por 100.000 habitantes e Padronização Direta de Mortalidade com a População Padrão da OMS.

### 6. Analítica de CSAP e Economia da Saúde
- Classificação completa dos **19 Grupos de Causas** da **Portaria MS/SAS nº 221/2008**.
- Taxa Bruta de CSAP por 10.000 habitantes:
  $$\text{Taxa Bruta CSAP} = \left( \frac{\sum_{i \in \text{CSAP}} N_i}{\text{População}} \right) \times 10.000$$
- Custos hospitalares evitáveis ($\sum \text{VAL\_TOT}$) e dias evitáveis ($\sum \text{DIAS\_PERM}$).
- Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária à Saúde:
  $$\text{ROI}_{\text{APS}} = \frac{(\alpha \cdot \text{Custo Evitável}) - \text{Investimento}_{\text{APS}}}{\text{Investimento}_{\text{APS}}}$$

### 7. Governança FAIR e Rastreabilidade Criptográfica
- Linhagem científica rastreável com manifestos **W3C PROV-O** em JSON-LD e hashes SHA-256 calculados no voo.
- Armazenamento particionado em **Apache Hive-Parquet** com suporte a consultas históricas reproduzíveis bit a bit (`as_of_snapshot`).

---

## ⚡ Guia de Início Rápido (Quickstart)

### 1. Cientistas de Dados (Python / Polars / PyTorch)

```python
import brhealth
from brhealth import Engine
import torch

# 1. Validação IBGE e CSAP
dv = brhealth.calculate_ibge_dv("355030") # 8
assert brhealth.is_csap("J45.0") == True   # Asma é evitável na APS

# 2. Ingestão Colunar de AIH/SIH
engine = Engine()
sih_batch = engine.hospital_morbidity.fetch(jurisdiction="SP", year=2023, month=1)

# 3. Conversão Zero-Copy para Polars DataFrame
df = sih_batch.to_polars()
print(df.head())

# 4. Exportação Zero-Copy para Tensores PyTorch via DLPack
tensor = torch.from_dlpack(sih_batch)

# 5. Avaliação de Métricas de CSAP
metricas = engine.evaluate_csap(sih_batch, reference_population=12_000_000)
print(f"Custo Hospitalar Evitável: R$ {metricas['avoidable_cost']:.2f}")

# 6. Exportar Manifesto FAIR W3C PROV-O
sih_batch.export_fair_manifest("manifesto_extracao.jsonld")
```

---

### 2. Bioestatísticos e Epidemiologistas (R / Tidyverse / Arrow)

```r
library(reticulate)
library(arrow)
library(dplyr)

# 1. Conectar ao BRHealth no ambiente virtual
use_virtualenv("./.venv", required = TRUE)
brhealth <- import("brhealth")

# 2. Validações e CSAP
dv <- brhealth$calculate_ibge_dv("355030") # 8 (São Paulo)
assert_that(brhealth$is_csap("J45.0") == TRUE) # Asma

# 3. Ingestão e Arrow C Data Interface (Zero-Copy)
engine <- brhealth$Engine()
sih_batch <- engine$hospital_morbidity$fetch(jurisdiction = "SP", year = 2023L, month = 1L)

# Converte ponteiros Apache Arrow para tabela nativa no R sem cópia
ptrs <- sih_batch$to_arrow_pointers()
tabela_arrow <- arrow::ImportRecordBatch(ptrs[[1]], ptrs[[2]])

# 4. Análise com Dplyr
resumo <- as.data.frame(tabela_arrow) %>%
  filter(is_csap == TRUE) %>%
  count(grupo_csap, sort = TRUE)

print(head(resumo))
```

---

### 3. Gestores de Saúde e Vigilância Epidemiológica (CLI)

```bash
# Instalar a CLI globalmente
cargo install --path crates/brhealth-cli

# Validar município do IBGE
brhealth dv 355030

# Classificar internação sob a Portaria 221/2008
brhealth csap J45.0

# Avaliar ROI da Atenção Primária (ESF)
brhealth roi --avoidable-cost 500000 --investment 100000 --attributable-fraction 0.50

# Calcular APVP (Anos Potenciais de Vida Perdidos)
brhealth apvp 35 42 18 55 62 --cutoff 70 --population 100000

# Baixar, enriquecer e exportar Parquet do SIH (gravando em ~/.brhealth/data/)
brhealth fetch --source datasus_sih --uf RJ --year 2023 --month 3 --enrich-csap --out-parquet ~/.brhealth/data/sih_rj.parquet
```

---

### 4. Engenheiros de Sistemas (Rust / `brhealth-core`)

```rust
use brhealth_core::domain::analytics::csap::{classify_cid10, compute_primary_care_roi, CsapGroup};
use brhealth_core::domain::transforms::ibge::calculate_ibge_dv;
use brhealth_core::domain::spatial::h3::coord_to_h3_index;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Validação de DV do IBGE
    let dv = calculate_ibge_dv("355030")?;
    println!("São Paulo: 355030{}", dv);

    // Classificação de CSAP
    let group = classify_cid10("J45.0");
    assert_eq!(group, Some(CsapGroup::Asma));

    // Indexação H3
    let cell = coord_to_h3_index(-23.55052, -46.633308, 8)?;
    println!("Célula H3: {:#x}", cell);

    // Retorno sobre Investimento na APS
    let roi = compute_primary_care_roi(500_000.0, 100_000.0, 0.40)?;
    println!("ROI: {:.1}%", roi * 100.0);

    Ok(())
}
```

---

## Estrutura do Workspace Cargo

```text
brhealth/
├── Cargo.toml                         # Workspace raiz (LTO fat, opt-level 3)
├── docs/                              # Documentação oficial completa
│   ├── INSTALLATION.md                # Guia de instalação multiplataforma
│   ├── USAGE_GUIDE.md                 # Manual de utilização e fórmulas científicas
│   ├── EXTENDING_BRHEALTH.md          # Guia de extensão de fontes e SPI
│   └── architecture/                  # SDD e backlog arquitetural
├── bindings/
│   ├── cpp/include/brhealth.hpp       # Header C++20 RAII
│   └── jvm/BRHealthEngine.java        # Interface Java 21 Panama FFM
└── crates/
    ├── brhealth-core/                 # Domínio Puro, Inbound/Outbound Ports, SPI e 26 Fontes
    ├── brhealth-cli/                  # CLI nativo de alta performance
    ├── brhealth-ffi/                  # C-ABI plana e Arrow C Data Interface
    ├── brhealth-python/               # Bindings PyO3 com DLPack e Acessores Semânticos
    └── brhealth-jni/                  # Bindings Panama FFM para JVM
```

---

## Compilação e Validação

```bash
# Compilar todo o workspace em modo release
cargo build --release --workspace

# Executar suíte completa de testes (100+ testes automatizados)
cargo test --workspace

# Executar doc-tests executáveis
cargo test --workspace --doc

# Validar com Clippy estrito (Zero Warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Executar micro-benchmarks científicos com Criterion
cargo bench -p brhealth-core
```

---

## Licenciamento e Direitos Autorais

```text
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
```

O projeto é licenciado sob a **GNU Affero General Public License v3 (AGPLv3)** com modelo de duplo licenciamento comercial exclusivo do criador.
