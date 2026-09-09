<div align="center">

# BRHealth

**Motor Analítico Colunar de Alta Performance para Saúde Coletiva e Determinantes Sociais**

[![CI/CD](https://github.com/MarcelDevBr/brhealth/actions/workflows/ci.yml/badge.svg)](https://github.com/MarcelDevBr/brhealth/actions)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_3.0--or--later-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust: 2024 Edition](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![Apache Arrow](https://img.shields.io/badge/Apache_Arrow-53.0-brightgreen.svg)](https://arrow.apache.org/)
[![W3C PROV-O](https://img.shields.io/badge/FAIR-W3C_PROV--O-blueviolet.svg)](https://www.w3.org/TR/prov-o/)

</div>

---

## Visão Geral

O **BRHealth** é um motor analítico colunar de alta performance desenvolvido em Rust para ingestão, harmonização, análise bioestatística e modelagem econômica de dados do Sistema Único de Saúde (SUS) e determinantes sociais brasileiros.

Construído sob os princípios de **Hexagonal Data-Oriented Design (Hexagonal DOD)**, memória contígua em **Apache Arrow** e interoperabilidade **Zero-Copy FFI** (Arrow C Data Interface), o BRHealth processa microdados brutos do DATASUS (como arquivos legados `.dbc` descompactados 100% nativamente) em centenas de megabytes por segundo, sem chamadas externas de shell e sem ponteiros intermediários inseguros.

---

## Arquitetura: Hexagonal Data-Oriented Design (Hexagonal DOD)

```text
               +-------------------------------------------------------------+
               |                       INBOUND PORTS                         |
               |       (CLI / REST / Python C-API / Scientific Pipelines)    |
               +------------------------------+------------------------------+
                                              |
                                              v
+------------------------------------------------------------------------------------------+
|                                     DOMAIN CORE                                          |
|                               (Puro - Zero I/O, Zero Unwraps)                            |
|                                                                                          |
|   +-----------------------+   +-----------------------+   +--------------------------+   |
|   |   Canonical Schemas   |   |   Spatial Indexing    |   |     IBGE Harmonizer      |   |
|   |     (Apache Arrow)    |   |       (Uber H3)       |   |   (Luhn Modulo 10 DV)    |   |
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
               |   (TransportPort, DecompressorPort, CachePort, StoragePort) |
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
|   |           (Ponteiros FFI diretos para Python / C++20 / Java 21+ FFM)             |   |
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

### 4. Country Packs: Brasil (20 Fontes Nacionais) e Global (6 Fontes Supranacionais)
- **Country Pack Brasil (`pack_brasil`)**:
  - **DATASUS / MS**:
    - **SIM** (Sistema de Informações sobre Mortalidade)
    - **SINASC** (Sistema de Informações sobre Nascidos Vivos)
    - **SIH** (Sistema de Informações Hospitalares - AIH Reduzida)
    - **SINAN** (Sistema de Informação de Agravos de Notificação)
    - **SIASUS** (Sistema de Informações Ambulatoriais - BPA/APAC)
    - **CNES** (Cadastro Nacional de Estabelecimentos de Saúde e Leitos)
    - **SI-PNI / RNDS** (Vigilância Imunológica e Vacinas)
    - **SISVAN** (Vigilância Alimentar e Nutricional)
    - **SISCAN / SISCOLO / SISMAMA** (Rastreamento de Câncer de Mama e Colo)
    - **BPS / CMED / Anvisa** (Banco de Preços em Saúde e Fármacos)
  - **IBGE & MDS**:
    - **IBGE Censo** (Censo Demográfico e Setores Censitários)
    - **IBGE PNAD** (PNAD Contínua - Rendimento e Condições de Vida)
    - **IBGE POF** (Pesquisa de Orçamentos Familiares e Gastos em Saúde)
    - **IBGE PeNSE** (Pesquisa Nacional de Saúde do Escolar)
    - **IBGE MUNIC** (Perfil e Capacidade da Gestão Municipal em Saúde)
    - **CadÚnico / MDS** (Vulnerabilidade Social e Transferência de Renda)
  - **Clima, Ambiente e Saneamento**:
    - **INMET** (Estações Meteorológicas de Superfície)
    - **BDQueimadas / INPE** (Focos de Calor por Satélite e Dispersão de Fumaça)
    - **PRODES / INPE** (Taxas de Desmatamento Anual e Risco de Zoonoses)
    - **SISAGUA / SNIS** (Qualidade da Água Potável e Saneamento)
- **Country Pack Global (`pack_global`)**:
  - **WHO GHO** (Global Health Observatory - Indicadores Globais da OMS / ODS 3)
  - **IHME GBD** (Global Burden of Disease - DALYs, YLLs, YLDs)
  - **Copernicus ERA5** (Reanálise Climática e Meteorológica Global em Grade)
  - **WorldPop** (Demografia e População Georreferenciada em Grade Contínua de 100m)
  - **PAHO / OPAS PLISA** (Vigilância Pan-Americana Transfronteiriça de Arboviroses)
  - **OpenAQ** (Monitoramento Global de Poluentes Atmosféricos e Qualidade do Ar)

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

### 7. Governança FAIR, Time-Travel e Persistência Híbrida
- Linhagem científica rastreável com manifestos **W3C PROV-O** em JSON-LD e hashes SHA-256 calculados no voo.
- Armazenamento particionado em **Apache Hive-Parquet** com suporte a consultas históricas reproduzíveis bit a bit (`as_of_snapshot`).
- Backend relacional embutido **SQLite Sync State** (`SqliteSyncState`) e persistência atômica em disco (`DiskSyncState`) para auditoria determinística.

### 8. Ecossistema Cross-Language e Interoperabilidade Zero-Copy
- **`crates/brhealth-core`**: Núcleo analítico puro, 26 fontes oficiais, schemas Arrow, CSAP, APVP, H3/S2, FAIR e decodificadores nativos.
- **`crates/brhealth-cli`**: Interface de linha de comando com subcomandos para cálculo de DV, CSAP, ROI, APVP, S2 e ingestão colunar completa.
- **`crates/brhealth-ffi`**: Exportação plana C-ABI e Arrow C Data Interface para integração binária universal.
- **`crates/brhealth-python`**: Bindings idiomáticos via PyO3 com acessores semânticos, suporte Arrow PyCapsule e **DLPack** para tensores PyTorch.
- **`crates/brhealth-jni`**: Bindings de alta performance para Java 21+ Project Panama (Foreign Function & Memory API).
- **`bindings/cpp/include/brhealth.hpp`**: Wrapper moderno C++20 com RAII sobre a C-ABI.

### 9. Harmonização Ontológica e Normalização Semântica
- Mapeamento transversal histórico **CID-9 $\leftrightarrow$ CID-10** e transição **CID-10 $\leftrightarrow$ CID-11** (OMS).
- Mapeador de conceitos **SNOMED-CT** para interoperabilidade FHIR R4.
- Tabela de Procedimentos do SUS (**SIGTAP**) de 10 dígitos e identificação de eventos sentinela evitáveis (amputações, diálise).
- Vocabulários Farmacêuticos Internacionais: Classificação **ATC** da OMS e mapeamento **RxNorm**.
- Validação estrita de consistência biológica (incompatibilidades anatômicas de sexo e faixas etárias extremas).

### 10. Extensibilidade SPI e Fontes Declarativas
- Loader dinâmico de manifestos declarativos **YAML/JSON** (`DeclarativeDataSource`), permitindo adicionar novas fontes sem recompilar o motor.
- Decodificador vetorial nativo **Apache GeoArrow** (EPSG:4326).
- Decodificador colunar para matrizes e grades climáticas **NetCDF / ERA5**.
- Clientes assíncronos **Tokio FTP** (DATASUS) e **HTTP Streaming** com *stream hashing* SHA-256 no voo.

---

## Estrutura do Workspace Cargo

```text
brhealth/
├── Cargo.toml                         # Workspace raiz (LTO fat, opt-level 3)
├── docs/architecture/                 # SDD, Especificações e Casos de Uso
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

## Instalação e Execução

### Pré-requisitos
- Rust 1.85+ (Edição 2024).
- Python 3.10+ (opcional para bindings Python).
- JDK 21+ (opcional para bindings Java Panama).

### Compilação e Testes

```bash
# Compilar todo o workspace em modo release
cargo build --release --workspace

# Executar suíte completa de testes (100 testes automatizados)
cargo test --workspace

# Validar com Clippy estrito (Zero Warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Executar micro-benchmarks científicos com Criterion
cargo bench --workspace
```

---

## Exemplos de Uso

### Python (Polars / PyArrow / PyTorch Zero-Copy)

```python
import brhealth

# 1. Validação e cálculo canônico do IBGE
dv = brhealth.calculate_ibge_dv("355030")
code_7 = brhealth.harmonize_ibge_code("355030")
print(f"Município: {code_7} (DV: {dv})") # 3550308 (DV: 8)

# 2. Vigilância de CSAP (Portaria MS/SAS 221/2008)
if brhealth.is_csap("J45.0"):
    group_id = brhealth.classify_cid10("J45.0")
    print(f"Internação evitável identificada: Grupo {group_id} (Asma)")

# 3. Bioestatística: Anos Potenciais de Vida Perdidos (APVP / YLL)
apvp = brhealth.compute_apvp([35, 42, 18, 55], cutoff_age=70)
print(f"Total de APVP: {apvp} anos") # 130 anos

# 4. Mapeamento Transversal Histórico CID-9 -> CID-10 e CID-10 -> CID-11
cid10 = brhealth.map_icd9_to_icd10("493") # J45 (Asma)
cid11 = brhealth.map_icd10_to_icd11("I10") # BA00 (Hipertensão essencial)

# 5. Motor Analítico com 26 Fontes Oficiais e Acessores Semânticos
engine = brhealth.Engine()
print(f"Fontes ativas no catálogo: {engine.source_count()}") # 26

# Ingestão com autocomplete idiomático:
# sih_batch = engine.hospital_morbidity.fetch(jurisdiction="3509502", years=[2022, 2023, 2024])
# polars_df = sih_batch.to_polars()
```

### Rust (Domínio Puro)

```rust
use brhealth_core::domain::analytics::csap::{classify_cid10, compute_primary_care_roi, CsapGroup};
use brhealth_core::domain::transforms::ibge::calculate_ibge_dv;
use brhealth_core::domain::spatial::coord_to_h3_index;

fn main() {
    // 1. Cálculo de Dígito Verificador IBGE
    let dv = calculate_ibge_dv("355030").unwrap();
    println!("São Paulo: 355030{}", dv); // 3550308

    // 2. Indexação H3 de Coordenadas
    let h3_cell = coord_to_h3_index(-23.55052, -46.633308, 8).unwrap();
    println!("Praça da Sé H3 (Res 8): {:#x}", h3_cell);

    // 3. Classificação de CSAP (Portaria 221/2008)
    let group = classify_cid10("J45.0");
    assert_eq!(group, Some(CsapGroup::Asma));

    // 4. ROI da Atenção Primária
    let roi = compute_primary_care_roi(500_000.0, 100_000.0, 0.40).unwrap();
    println!("ROI da Atenção Primária: {:.1}%", roi * 100.0); // 100.0%
}
```

### C++20 Moderno

```cpp
#include "brhealth.hpp"
#include <iostream>

int main() {
    std::cout << "BRHealth Engine Version: " << brhealth::version() << "\n";
    uint8_t dv = brhealth::calculate_ibge_dv("355030");
    std::cout << "DV São Paulo: " << static_cast<int>(dv) << "\n";
    auto csap = brhealth::classify_csap("J45");
    if (csap) {
        std::cout << "Grupo CSAP: " << static_cast<int>(*csap) << "\n";
    }
    return 0;
}
```

---

## Licenciamento e Direitos Autorais

Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.

O projeto é licenciado sob a **GNU Affero General Public License v3 (AGPLv3)** com modelo de duplo licenciamento comercial exclusivo do criador.
