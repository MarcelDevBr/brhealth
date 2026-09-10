<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Manual de Utilização - BRHealth

O **BRHealth** é um motor analítico colunar de alta performance para dados do Sistema Único de Saúde (SUS), determinantes sociais e saúde coletiva. Este manual fornece exemplos práticos, sintaxe detalhada e fluxos de trabalho completos para as seguintes interfaces:

1. [Interface de Linha de Comando (CLI)](#1-interface-de-linha-de-comando-cli)
2. [Guia para Ciência de Dados em Python (Polars / PyArrow / PyTorch)](#2-guia-para-ciência-de-dados-em-python)
3. [Desenvolvimento em Rust (Biblioteca `brhealth-core`)](#3-desenvolvimento-em-rust-biblioteca-brhealth-core)
4. [Interoperabilidade C++20 e Java 21+ Project Panama](#4-interoperabilidade-c20-e-java-21-project-panama)
5. [Fundamentação Científica e Formulações Matemáticas](#5-fundamentação-científica-e-formulações-matemáticas)

---

## 1. Interface de Linha de Comando (CLI)

O binário `brhealth` oferece ferramentas analíticas completas acessíveis diretamente no terminal.

### 1.1 Resumo de Comandos Disponíveis

| Subcomando | Descrição |
| :--- | :--- |
| `brhealth version` | Exibe a versão, arquitetura e metadados de licenciamento. |
| `brhealth sources` | Lista todas as 26 fontes oficiais registradas (Pacote Brasil e Pacote Global). |
| `brhealth dv <CODE>` | Calcula e valida o Dígito Verificador (DV) do IBGE pelo algoritmo Luhn Módulo 10. |
| `brhealth csap <CID>` | Classifica um código CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008. |
| `brhealth roi` | Calcula o Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária. |
| `brhealth apvp <AGES...>` | Calcula Anos Potenciais de Vida Perdidos (APVP / YLL) e taxa por 100.000 hab. |
| `brhealth s2` | Converte coordenadas geográficas em índice Google S2 CellId de 64 bits. |
| `brhealth cid9 <CODE>` | Mapeia código histórico da CID-9 para o equivalente canônico na CID-10. |
| `brhealth clean-cache` | Limpa os caches de dados e estados temporários do motor analítico. |
| `brhealth fetch` | Executa o pipeline colunar completo (ingestão, harmonização, H3, CSAP e Parquet). |

---

### 1.2 Exemplos Práticos no Terminal

#### A. Verificação de Fontes Disponíveis
```bash
brhealth sources
```
*Saída resumida:*
```text
┌──────────────────────┬────────────────────────────────────────────┬──────────────────┬──────────────────────┐
│ IDENTIFICADOR        │ NOME DA FONTE                              │ ÓRGÃO EMISSOR    │ RESOLUÇÃO TEMPORAL   │
├──────────────────────┼────────────────────────────────────────────┼──────────────────┼──────────────────────┤
│ --- PACOTE BRASIL (DATASUS / IBGE / MINISTÉRIO DA SAÚDE) ------------------------------------------------- │
│ datasus_sim          │ Sistema de Informações sobre Mortalidade   │ DATASUS / MS     │ Anual (1979-2024)    │
│ datasus_sinasc       │ Sistema de Nascidos Vivos                  │ DATASUS / MS     │ Anual (1994-2024)    │
│ datasus_sih          │ Sistema de Informações Hospitalares (SIH)  │ DATASUS / MS     │ Mensal (1992-2024)   │
│ datasus_sinan        │ Sistema de Notificação de Agravos (SINAN)  │ DATASUS / MS     │ Anual (2000-2024)    │
│ datasus_siasus       │ Sistema de Ambulatorial do SUS (SIA-SUS)   │ DATASUS / MS     │ Mensal (1994-2024)   │
...
Total de fontes registradas prontas para consulta: 26
```

#### B. Validação Territorial de Município do IBGE
```bash
# Código com 6 dígitos de São Paulo (355030)
brhealth dv 355030
```
*Saída:*
```text
=== Validação Territorial IBGE (Luhn Módulo 10) ===
Código original:   355030
Dígito Verificador (DV): 8
Código harmonizado (7 dígitos): 3550308
Status: ✓ CÓDIGO MUNICIPAL VÁLIDO CONFORME PADRÃO IBGE
```

#### C. Classificação de Internações Sensíveis à Atenção Primária (CSAP)
```bash
# Asma (J45.0)
brhealth csap J45.0
```
*Saída:*
```text
=== Classificação Epidemiológica de Internação (Portaria MS/SAS nº 221/2008) ===
Código CID-10: J45.0
Classificação: CONDIÇÃO SENSÍVEL À ATENÇÃO PRIMÁRIA (CSAP)
Grupo 7: Asma
Status: ✓ INTERNAÇÃO HOSPITALAR POTENCIALMENTE EVITÁVEL NA APS
```

#### D. Avaliação de Retorno sobre Investimento (ROI da Atenção Primária)
```bash
# Custo evitável: R$ 500.000, Investimento APS: R$ 100.000, Fração atribuível: 50%
brhealth roi --avoidable-cost 500000 --investment 100000 --attributable-fraction 0.50
```
*Saída:*
```text
┌───────────────────────────────────────────────────────────────────────────┐
│ Avaliação de Economia da Saúde e Retorno sobre Investimento (APS/ESF)     │
├───────────────────────────────────────────────────────────────────────────┤
│ • Custo Hospitalar Evitável Direto:   R$        500000.00                 │
│ • Investimento Orçamentário na APS:   R$        100000.00                 │
│ • Fração Atribuível Epidemiológica:                  50.0%                │
│ • Economia Líquida Estimada ao SUS:   R$        250000.00                 │
│ • ROI (Retorno sobre Investimento):                 150.00%                │
│ Status: ✓ ECONÔMICAMENTE SUPERAVITÁRIO (Gera valor líquido ao erário)     │
└───────────────────────────────────────────────────────────────────────────┘
```

#### E. Cálculo de Mortalidade Prematura (APVP / YLL)
```bash
# Idades de óbito de amostra com corte padrão de 70 anos e população de 100.000 hab.
brhealth apvp 35 42 18 55 62 --cutoff 70 --population 100000
```
*Saída:*
```text
=== Anos Potenciais de Vida Perdidos (APVP / YLL) ===
Idades de óbito fornecidas: [35, 42, 18, 55, 62]
Idade de corte prematuro:   70 anos
Total de APVP calculado:    138 anos perdidos
População de referência:    100000 habitantes
Taxa de APVP padronizada:   138.00 por 100.000 hab.
```

#### F. Pipeline Colunar Completo com Exportação Parquet
```bash
# Consulta ao SIH de Roraima (RR), competência 2023, enriquecendo CSAP e gerando Parquet
brhealth fetch --source datasus_sih --uf RR --year 2023 --month 5 --enrich-csap --out-parquet /tmp/sih_rr_2023_05.parquet
```

---

## 2. Guia para Ciência de Dados em Python

A biblioteca `brhealth` oferece integração nativa com **Polars**, **PyArrow**, **Pandas** e **PyTorch**, operando sob a arquitetura **Zero-Copy** através do protocolo **Arrow PyCapsule** e **DLPack**.

### 2.1 Importação e Uso Rápido

```python
import brhealth

# 1. Validação canônica do IBGE
dv = brhealth.calculate_ibge_dv("355030") # Retorna 8
municipio_canônico = brhealth.harmonize_ibge_code("355030") # "3550308"

# 2. Avaliação de CSAP (Portaria 221/2008)
assert brhealth.is_csap("J45.0") == True
grupo_id = brhealth.classify_cid10("J45.0") # Grupo 7 (Asma)

# 3. Transição de Ontologias Médicas
cid10 = brhealth.map_icd9_to_icd10("493")     # "J45" (Asma)
cid11 = brhealth.map_icd10_to_icd11("I10")    # "BA00" (Hipertensão)
snomed = brhealth.map_icd10_to_snomed("I10")  # "38341003" (Hipertensão Essencial)

# 4. Verificação de Procedimentos SIGTAP
if brhealth.is_amputation_procedure("0407040011"):
    print("Alerta: Procedimento de amputação de membro detectado.")
```

---

### 2.2 Consultas e Extrações com o `Engine()`

O motor `Engine()` centraliza o acesso às 26 fontes de dados de saúde e determinantes sociais com acessores semânticos:

```python
from brhealth import Engine
import polars as pl
import torch

engine = Engine()
print(f"Total de fontes ativas: {engine.source_count()}")

# Ingestão do SIH (Morbidade Hospitalar) de SP para múltiplos anos
sih_batch = engine.hospital_morbidity.fetch(
    jurisdiction="SP",
    year=2023,
    month=1,
    harmonize_ibge=True,
    enrich_csap=True,
    assign_h3=8 # Indexa coordenadas em resolução Uber H3 8 (~0.7 km²)
)

print(f"Linhas recuperadas: {sih_batch.num_rows}")
print(f"Colunas do esquema: {sih_batch.column_names}")

# 1. Conversão Zero-Copy para Polars DataFrame
df = sih_batch.to_polars()
print(df.head())

# 2. Conversão Zero-Copy para PyArrow Table / RecordBatch
arrow_table = sih_batch.to_pyarrow()

# 3. Conversão Zero-Copy para PyTorch via DLPack
# Transfere buffers de memória contígua diretamente para tensores
tensor = torch.from_dlpack(sih_batch)

# 4. Avaliação de Métricas de CSAP e Custos Evitáveis
metricas = engine.evaluate_csap(sih_batch, reference_population=12_000_000)
print("Internações CSAP:", metricas["csap_admissions"])
print("Proporção CSAP:", metricas["csap_proportion"], "%")
print("Custo Hospitalar Evitável: R$", metricas["avoidable_cost"])

# 5. Exportação de Auditoria Científica FAIR (W3C PROV-O JSON-LD)
sih_batch.export_fair_manifest("manifesto_extracao.jsonld")
```

---

### 2.3 Acessores Semânticos Adicionais

```python
# A. Estatísticas Vitais (SIM ou SINASC)
sim_batch = engine.vital_statistics.fetch(source="SIM", jurisdiction="RJ", year=2022)
sinasc_batch = engine.vital_statistics.fetch(source="SINASC", jurisdiction="MG", year=2022)

# B. Notificações Epidemiológicas (SINAN)
dengue_batch = engine.notifications.fetch(disease="DENG", jurisdiction="BA", year=2023)

# C. Reanálise Climática e Variáveis Ambientais (Copernicus ERA5)
climate_batch = engine.global_climate.fetch_reanalysis(jurisdiction="BRA", year=2023, month=6)
```

---

## 3. Desenvolvimento em Rust (Biblioteca `brhealth-core`)

Adicione a dependência ao seu `Cargo.toml`:

```toml
[dependencies]
brhealth-core = { path = "crates/brhealth-core" }
arrow = "53.0"
tokio = { version = "1.0", features = ["full"] }
```

### 3.1 Exemplo Completo de Análise Epidemiológica

```rust
use std::sync::Arc;
use brhealth_core::domain::analytics::csap::{
    classify_cid10, compute_csap_metrics, compute_primary_care_roi, CsapGroup
};
use brhealth_core::domain::analytics::mortality::{compute_apvp, compute_apvp_rate};
use brhealth_core::domain::transforms::ibge::{calculate_ibge_dv, harmonize_ibge_code};
use brhealth_core::domain::transforms::ontology::{Icd10Code, MedicalOntologyHarmonizer};
use brhealth_core::domain::spatial::h3::coord_to_h3_index;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validação territorial do IBGE (Luhn Módulo 10)
    let raw_ibge = "355030";
    let dv = calculate_ibge_dv(raw_ibge)?;
    let codigo_harmonizado = harmonize_ibge_code(raw_ibge)?;
    println!("Município canônico: {} (DV: {})", codigo_harmonizado, dv);

    // 2. Ontologia Médica Tipada e Zero-Alocação
    let code = Icd10Code::parse("J45.0")?;
    println!("Código: {}, Capítulo: {:?}", code.as_str(), code.chapter());

    // 3. Classificação de CSAP (Portaria MS/SAS nº 221/2008)
    if let Some(grupo) = classify_cid10("J45.0") {
        assert_eq!(grupo, CsapGroup::Asma);
        println!("CSAP detectada: Grupo {} ({})", grupo.id(), grupo.name());
    }

    // 4. Cálculo de Retorno sobre Investimento da Atenção Primária
    let custo_evitavel = 1_000_000.0;
    let investimento_aps = 200_000.0;
    let fracao_atribuivel = 0.50; // 50% de eficácia da APS
    let roi = compute_primary_care_roi(custo_evitavel, investimento_aps, fracao_atribuivel)?;
    println!("ROI da APS: {:.1}%", roi * 100.0); // 150.0%

    // 5. Mortalidade Prematura (APVP / YLL)
    let idades = vec![25, 34, 49, 18, 61];
    let total_apvp = compute_apvp(&idades, 70);
    let taxa_apvp = compute_apvp_rate(total_apvp, 50_000)?;
    println!("Total APVP: {} anos perdidos | Taxa: {:.2} por 100k hab.", total_apvp, taxa_apvp);

    // 6. Indexação Espacial Uber H3
    let lat = -23.55052;
    let lon = -46.633308;
    let h3_index = coord_to_h3_index(lat, lon, 8)?;
    println!("Célula H3 Res 8: {:#x}", h3_index);

    Ok(())
}
```

---

## 4. Interoperabilidade C++20 e Java 21+ Project Panama

### 4.1 C++20 Moderno com RAII (`bindings/cpp/include/brhealth.hpp`)

O wrapper C++20 oferece conformidade total com o padrão RAII (*Resource Acquisition Is Initialization*), liberando ponteiros do Apache Arrow automaticamente.

```cpp
#include "bindings/cpp/include/brhealth.hpp"
#include <iostream>

int main() {
    std::cout << "BRHealth Version: " << brhealth::version() << std::endl;

    // 1. Validação IBGE
    uint8_t dv = brhealth::calculate_ibge_dv("355030");
    std::cout << "DV São Paulo: " << static_cast<int>(dv) << std::endl;

    // 2. Classificação de CSAP
    auto csap = brhealth::classify_csap("J45.0");
    if (csap.has_value()) {
        std::cout << "Grupo CSAP: " << static_cast<int>(*csap) << std::endl;
    }

    // 3. Indexação H3
    uint64_t cell_h3 = brhealth::latlng_to_h3(-23.55052, -46.633308, 8);
    std::cout << "H3 Index: 0x" << std::hex << cell_h3 << std::dec << std::endl;

    // 4. Ingestão Colunar com Arrow C Data Interface
    brhealth::Client client;
    brhealth::ArrowRecordBatch batch = client.fetch_mortality("SP", 2022);
    std::cout << "Linhas carregadas via C Data Interface: " << batch.num_rows() << std::endl;

    return 0;
}
```

---

### 4.2 Java 21+ Project Panama FFM (`bindings/jvm/BRHealthEngine.java`)

A nova API de Memória e Funções Estrangeiras do Java 21 (*Foreign Function & Memory API*) elimina o overhead de JNI, permitindo passagens diretas de memória nativa:

```java
import br.health.BRHealthEngine;

public class Main {
    public static void main(String[] args) throws Throwable {
        String libPath = "target/release/libbrhealth_jni.so"; // ou .dylib / .dll

        try (BRHealthEngine engine = new BRHealthEngine(libPath)) {
            // 1. Cálculo de DV do IBGE
            int dv = engine.calculateIbgeDv("355030");
            System.out.println("DV São Paulo: " + dv); // 8

            // 2. Classificação de CSAP
            int grupoCsap = engine.classifyCsap("J45.0");
            System.out.println("Grupo CSAP: " + grupoCsap); // 7

            // 3. Indexação H3
            long h3Index = engine.latLngToH3(-23.55052, -46.633308, (byte) 8);
            System.out.printf("H3 Cell ID: 0x%x%n", h3Index);

            // 4. Retorno sobre Investimento
            double roi = engine.computePrimaryCareRoi(500000.0, 100000.0, 0.50);
            System.out.printf("ROI APS: %.1f%%%n", roi * 100.0); // 150.0%
        }
    }
}
```

---

## 5. Fundamentação Científica e Formulações Matemáticas

Todas as transformações, taxas e modelos econômicos do BRHealth são estritamente documentados com as seguintes definições analíticas formais:

### 5.1 Anos Potenciais de Vida Perdidos (APVP / YLL)

Os Anos Potenciais de Vida Perdidos mensuram o impacto social e epidemiológico das mortes prematuras:

$$\text{APVP} = \sum_{i=1}^{n} d_i \cdot (L - a_i) \quad \forall \; a_i < L$$

Onde:
- $L$: Idade de corte para morte prematura (canonicamente $L = 70$ anos no Brasil, ou conforme tábua de vida da OMS).
- $a_i$: Idade do indivíduo no momento do óbito.
- $d_i$: Número de óbitos ocorridos na idade $a_i$.

A **Taxa Padronizada de APVP** por 100.000 habitantes é expressa por:

$$\text{Taxa APVP} = \left( \frac{\text{APVP}}{\text{População de Referência}} \right) \times 100.000$$

---

### 5.2 Taxa Bruta de CSAP (Portaria MS/SAS nº 221/2008)

A Taxa de Internações por Condições Sensíveis à Atenção Primária por 10.000 habitantes é dada por:

$$\text{Taxa Bruta CSAP} = \left( \frac{\sum_{i \in \text{CSAP}} N_i}{\text{População}} \right) \times 10.000$$

Onde $\sum_{i \in \text{CSAP}} N_i$ representa o número total de internações classificadas nos 19 grupos de causas evitáveis no período.

---

### 5.3 Retorno sobre Investimento em Saúde Coletiva na APS ($\text{ROI}_{\text{APS}}$)

Avalia a eficiência orçamentária dos investimentos na Estratégia Saúde da Família (ESF) frente aos custos hospitalares diretos evitáveis:

$$\text{ROI}_{\text{APS}} = \frac{(\alpha \cdot \text{Custo Evitável}) - \text{Investimento}_{\text{APS}}}{\text{Investimento}_{\text{APS}}}$$

Onde:
- $\text{Custo Evitável}$: Custo total hospitalar direto gerado por internações CSAP ($\sum \text{VAL\_TOT}$ do SIH-SUS).
- $\alpha$: Fração de impacto atribuível à Atenção Primária ($0.0 < \alpha \le 1.0$, tipicamente estimada entre $0.30$ e $0.60$ em ensaios econômicos).
- $\text{Investimento}_{\text{APS}}$: Montante orçamentário liquidado no período na Atenção Básica.

---

### 5.4 Dígito Verificador do IBGE (Algoritmo Luhn Módulo 10)

Para um código municipal de 6 dígitos $C = d_1 d_2 d_3 d_4 d_5 d_6$, os pesos alternados são $w_i = (1, 2, 1, 2, 1, 2)$:

$$p_i = d_i \times w_i$$

$$s_i = \begin{cases} p_i, & \text{se } p_i < 10 \\ \lfloor p_i / 10 \rfloor + (p_i \pmod{10}), & \text{se } p_i \ge 10 \end{cases}$$

$$S = \sum_{i=1}^{6} s_i$$

$$\text{DV} = (10 - (S \pmod{10})) \pmod{10}$$
