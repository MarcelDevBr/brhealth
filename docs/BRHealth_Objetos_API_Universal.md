<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Guia Unificado da API e Resolução de Fontes com Cache — BRHealth (Agnóstico à Linguagem)

Esta documentação descreve a **API conceitual, o modelo de resolução transparente de dados e o catálogo de fontes** do **BRHealth**, de forma **independente de linguagem de programação** (Rust, Python, C/C++, Java, R).

---

## 1. Princípio Arquitetural: Interface Única e Transparente

No BRHealth, **o usuário e as bibliotecas clientes interagem com as fontes de dados através de uma interface conceitual idêntica e unificada**, independentemente de onde o dado físico resida. 

O consumidor da API **não gerencia manualmente arquivos locais baixados (`.dbc`, `.dbf`, `.csv`)**, caminhos temporários no sistema de arquivos ou fluxos de download manuais. 

### 1.1. Política de Atualização e Cache do Core

Por padrão, a política do BRHealth é garantir **dados oficiais sempre atualizados**:
- **`force_download = true` por padrão**: Toda consulta acessa a fonte oficial remota, garante que o dado mais recente seja obtido e atualiza o cache local.
- **Cache como contingência e aceleração explícita**: Se o usuário desejar usar a cópia local já existente para máxima velocidade (offline / sem tocar na rede), ele pode configurar `use_cache = true` (ou `force_download = false`). Em caso de falha de rede ou queda do DATASUS, o core recorre automaticamente à versão salva no cache local (*stale fallback*).
- **Limpeza do Cache**: O usuário pode solicitar explicitamente a invalidação/limpeza do cache a qualquer momento (total, por fonte ou por data/dias de antiguidade).

```mermaid
flowchart TD
    REQ["Consulta: engine.fetch(source_id, jurisdiction, year, month)"] --> CHECK_FORCE{"force_download ativo?<br/>(Padrão: TRUE)"}
    
    CHECK_FORCE -- Sim (Padrão) --> FETCH_REMOTE["1. Ingestão Remota Oficial (FTP / HTTP / API)"]
    CHECK_FORCE -- Não (use_cache=True) --> CHECK_CACHE{"2. O dado existe no Cache Local Hive-Parquet?"}
    
    CHECK_CACHE -- Sim --> HIT["✓ Cache Hit: Leitura Colunar Zero-Copy imediata"]
    CHECK_CACHE -- Não --> FETCH_REMOTE
    
    FETCH_REMOTE --> RES_PRIMARY{"Fonte primária respondeu?"}
    RES_PRIMARY -- Sim --> STORE["Decodifica, persiste no Cache e atualiza Catálogo FAIR"]
    RES_PRIMARY -- Falha de Rede --> MIRRORS["3. Contingência: Servidores Mirrors / Espelhos"]
    
    MIRRORS -- Sucesso --> STORE
    MIRRORS -- Falha Total --> STALE_FALLBACK{"Há versão anterior gravada no Cache?"}
    
    STALE_FALLBACK -- Sim --> SERVE_STALE["⚠ Serve Snapshot Stale + Alerta de Degradação"]
    STALE_FALLBACK -- Não --> ERR["Retorna Erro Tipado de Fonte Indisponível"]
    
    STORE --> OUT["Retorna RecordBatch Apache Arrow"]
    HIT --> OUT
    SERVE_STALE --> OUT
```

1. **Localização Canônica de Cache**: O cache particionado não usa pastas voláteis como `/tmp`. Ele reside na pasta persistente do usuário (`$HOME/.brhealth/cache` ou variável `BRHEALTH_CACHE_DIR`), particionado por fonte, UF e ano sob o padrão **Hive-Parquet** (`{dataset}/uf={UF}/year={ANO}/snapshot={UUID}/data.parquet`).
2. **Governança de Ciclo de Vida do Cache**: O core oferece operações explícitas para invalidar ou limpar o cache:
   - Limpeza total do cache ou de uma fonte específica (`engine.cache.clear(source_id)`).
   - Limpeza seletiva de dados mais antigos que uma data ou dias de corte (`engine.cache.clear_older_than(days=30)`).

---

## 2. Operações de Gestão de Cache no Core (`engine.cache`)

O motor disponibiliza um sub-objeto de gerenciamento de cache:

| Operação | Parâmetros | Descrição |
| :--- | :--- | :--- |
| **`clear()`** | `[source_id]` | Remove todos os dados em cache. Se `source_id` for informado, limpa apenas a partição daquela fonte (ex: `"datasus.sih"`). |
| **`clear_older_than()`** | `cutoff_date` ou `days` | Remove snapshots e arquivos de dados cujos carimbos de criação sejam anteriores à data ou quantidade de dias de corte (ex: `days=60` ou `cutoff="2025-01-01"`). |
| **`status()`** | `[source_id]` | Retorna o tamanho total ocupado em disco, contagem de snapshots salvos e intervalo de datas das fontes cacheadas. |

---

## 3. Catálogo Oficial de Fontes de Dados Integradas

O catálogo do BRHealth é pré-carregado no núcleo com mais de 25 fontes prontas para consulta automática:

### 3.1. Estatísticas Vitais e Assistenciais Nacionais (DATASUS / Ministério da Saúde)

| Identificador (`source_id`) | Acessador Semântico | Sistema / Conteúdo | Resolução Espacial | Temporalidade |
| :--- | :--- | :--- | :--- | :--- |
| **`datasus.sih`** | `engine.hospital_morbidity` | SIH-SUS: Internações Hospitalares (AIH Reduzida) | Município (IBGE) | Mensal (1998–2026) |
| **`datasus.sim`** | `engine.vital_statistics` | SIM-SUS: Declarações de Óbito e Mortalidade Geral | Município (IBGE) | Anual/Mensal (1996–2026) |
| **`datasus.sinasc`** | `engine.vital_statistics` | SINASC: Nascidos Vivos e Condições Perinatais | Município (IBGE) | Anual/Mensal (1996–2026) |
| **`datasus.sinan`** | `engine.notifications` | SINAN: Doenças e Agravos de Notificação Compulsória | Município (IBGE) | Mensal (2007–2026) |
| **`datasus.sia`** | `engine.ambulatory` | SIA-SUS: Produção Ambulatorial de Média/Alta Complexidade | Município (IBGE) | Mensal (2008–2026) |
| **`datasus.cnes`** | `engine.ambulatory` | CNES: Estabelecimentos, Leitos e Recursos Físicos | Estabelecimento | Mensal (2005–2026) |
| **`datasus.sipni`** | `engine.vital_statistics` | SI-PNI: Cobertura Vacinal e Imunizações | Município (IBGE) | Mensal (2010–2026) |
| **`datasus.sisvan`** | `engine.demographics` | SISVAN: Vigilância Alimentar e Nutricional | Município (IBGE) | Anual (2008–2026) |
| **`datasus.siscan`** | `engine.ambulatory` | SISCAN: Rastreamento do Câncer de Mama e Colo Uterino | Município (IBGE) | Anual (2013–2026) |
| **`datasus.bps`** | `engine.social` | BPS: Banco de Preços em Saúde (Medicamentos/Insumos) | Estado / Município | Mensal (2015–2026) |

### 3.2. Demografia, Orçamentos e Censo (IBGE)

| Identificador (`source_id`) | Acessador Semântico | Pesquisa / Conteúdo | Periodicidade |
| :--- | :--- | :--- | :--- |
| **`ibge.censo`** | `engine.demographics` | Censo Demográfico: População, idade, sexo, domicílios | Decenal (2010, 2022) |
| **`ibge.pnad`** | `engine.demographics` | PNAD Contínua: Condições socioeconômicas e de trabalho | Trimestral / Anual |
| **`ibge.pof`** | `engine.demographics` | POF: Despesas familiares com saúde e medicamentos | Quinquenal |
| **`ibge.pense`** | `engine.demographics` | PeNSE: Saúde Escolar e Comportamentos de Risco | Amostral |
| **`ibge.munic`** | `engine.demographics` | MUNIC: Estrutura da gestão de saúde nos municípios | Anual |

### 3.3. Clima, Ambiente e Desmatamento

| Identificador (`source_id`) | Acessador Semântico | Órgão / Conteúdo | Granularidade |
| :--- | :--- | :--- | :--- |
| **`environmental.inmet`** | `engine.environmental` | INMET: Estações Meteorológicas (Chuva, Temp, Umidade) | Coordenadas / Estação |
| **`environmental.bdqueimadas`** | `engine.environmental` | INPE: Focos de Calor e Queimadas via Satélite | Ponto Geográfico (Lat/Lon) |
| **`environmental.prodes`** | `engine.environmental` | INPE: Taxas de Desmatamento da Amazônia e Cerrado | Polígono / Município |
| **`environmental.sisagua`** | `engine.environmental` | Ministério da Saúde: Qualidade da Água de Abastecimento | Município |

### 3.4. Proteção Social (MDS)

| Identificador (`source_id`) | Acessador Semântico | Órgão / Conteúdo | Granularidade |
| :--- | :--- | :--- | :--- |
| **`mds.cadunico`** | `engine.social` | MDS: Cadastro Único de Famílias Vulneráveis | Município (IBGE) |

### 3.5. Supranacionais e Globais (Country Pack Global)

| Identificador (`source_id`) | Organização | Conteúdo | Granularidade |
| :--- | :--- | :--- | :--- |
| **`global.who_gho`** | OMS | Global Health Observatory: Indicadores mundiais de saúde | Países |
| **`global.ihme_gbd`** | IHME | Global Burden of Disease: Carga de doença (DALYs/YLLs) | Subnacional e Global |
| **`global.copernicus_era5`** | ECMWF | Copernicus ERA5: Reanálise climática de alta resolução | Grade Global 0.25° |
| **`global.worldpop`** | WorldPop | Grade populacional georreferenciada de alta resolução | Células 100m / 1km |
| **`global.paho_plisa`** | OPAS | Indicadores de saúde pública integrados das Américas | Américas |
| **`global.openaq`** | OpenAQ | Dados globais de poluentes atmosféricos ($PM_{2.5}$, $PM_{10}$, $O_3$) | Sensores / Estações |

---

## 4. Exemplos Conceituais de Uso da API

### 4.1. Consulta Unificada em Python

```python
import brhealth

# 1. Inicializa o motor analítico
engine = brhealth.Engine()

# 2. Chamada Padrão (force_download=True por padrão):
# O core busca na fonte oficial, atualiza o cache local Hive-Parquet e retorna
batch_sp = engine.hospital_morbidity.fetch(
    jurisdiction="SP",
    year=2024,
    month=1,
    harmonize_ibge=True  # padroniza municípios para 7 dígitos canônicos
)

# 3. Consulta offline ou acelerada por Cache (reutiliza dados salvos localmente):
batch_sp_cached = engine.hospital_morbidity.fetch(
    jurisdiction="SP",
    year=2024,
    month=1,
    force_download=False  # Reutiliza o cache existente sem tocar na rede
)

# 4. Gestão e Limpeza Explícita de Cache:
# Limpar registros do cache com mais de 60 dias:
engine.cache.clear_older_than(days=60)

# Limpar apenas os dados cacheados do SIH:
engine.cache.clear(source_id="datasus.sih")

# Limpeza completa de todo o cache local:
engine.cache.clear()

# 5. Interoperabilidade direta Zero-Copy para Ciência de Dados
df_polars = batch_sp.to_polars()
table_arrow = batch_sp.to_arrow()
tensor_torch = batch_sp.to_torch()
```

### 4.2. Consulta Unificada em Rust (`brhealth-core`)

```rust
use chrono::Utc;
use brhealth_core::domain::application::{BRHealthApplicationService, PipelineExecutionOptions};
use brhealth_core::domain::source_spi::{DataQueryParams, GeographicScope};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Instanciar o serviço de aplicação
    let app = BRHealthApplicationService::standard_in_memory()?;

    // 2. Parâmetros de consulta
    let params = DataQueryParams {
        scope: GeographicScope::National { iso_3166_alpha3: "BRA".into() },
        jurisdiction_code: Some("MG".into()),
        year: 2024,
        month: Some(1),
        extra_filters: Default::default(),
        as_of_snapshot: None,
    };

    // 3. Opções de execução (persistência e atualização automática)
    let options = PipelineExecutionOptions {
        force_download: true, // Padrão: busca na fonte oficial e atualiza cache
        persist_to_cache: true,
        harmonize_ibge: true,
        enrich_csap: true,
        ..Default::default()
    };

    // 4. Execução: busca oficial, persiste em cache e emite manifesto FAIR
    let result = app.execute_full_pipeline("datasus.sih", &params, &options).await?;
    println!("Total de registros: {}", result.batches[0].num_rows());

    // 5. Limpeza explícita do cache quando desejado
    app.clear_cache(Some("datasus.sih"))?;

    Ok(())
}
```

---

## 5. Estrutura Canônica de Dados e Metadados do Objeto Retornado

Independentemente da fonte consultada, a saída é encapsulada em um contêiner colunar contíguo **Apache Arrow `RecordBatch`**, contendo:

1. **Schema Rigoroso de Tipos**: Tipagem forte (inteiros de 32/64 bits, floats, timestamps, strings UTF-8 ou dicionários categóricos) sem perda de precisão ou truncamento.
2. **Harmonização Territorial Embutida**: Colunas de municípios (`MUNIC_RES`, `MUNIC_MOV`, etc.) padronizadas em 7 dígitos canônicos oficiais do IBGE com DV calculado via Luhn Módulo 10.
3. **Manifesto FAIR W3C PROV-O (`manifest`)**:
   - `raw_sha256`: Hash criptográfico SHA-256 do arquivo original no órgão emissor.
   - `created_at_utc`: Carimbo ISO-8601 exato do processamento.
   - `transformations`: Grafo de operações aplicadas (descompressão Blast, decodificação DBF, CSAP, H3).
