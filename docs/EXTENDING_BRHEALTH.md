<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Guia de Extensão e Desenvolvimento de Novas Fontes - BRHealth

Este documento é o guia definitivo para arquitetos, engenheiros de software e pesquisadores que desejam **estender** o motor analítico BRHealth. Aqui você aprenderá como implementar novos adaptadores de dados governamentais, criar *Country Packs* para outros países, adicionar decodificadores de novos formatos e estender as portas de infraestrutura.

---

## Índice

1. [Arquitetura Hexagonal DOD e Filosofia de Extensão](#1-arquitetura-hexagonal-dod-e-filosofia-de-extensão)
2. [O Contrato SPI: `HealthDataSourceSPI`](#2-o-contrato-spi-healthdatasourcespi)
3. [Tutorial Passo a Passo: Implementando um Novo Adaptador](#3-tutorial-passo-a-passo-implementando-um-novo-adaptador)
   - [Passo 1: Definir os Esquemas Canônicos e Metadados](#passo-1-definir-os-esquemas-canônicos-e-metadados)
   - [Passo 2: Implementar a Resolução de Localizadores (`resolve_locator`)](#passo-2-implementar-a-resolução-de-localizadores-resolve_locator)
   - [Passo 3: Decodificação Colunar Vetorizada (`fetch_and_decode`)](#passo-3-decodificação-colunar-vetorizada-fetch_and_decode)
   - [Passo 4: Registro no `CountryPack` ou `SourceRegistry`](#passo-4-registro-no-countrypack-ou-sourceregistry)
4. [Fontes Declarativas via Manifestos YAML/JSON (Sem Recompilar)](#4-fontes-declarativas-via-manifestos-yamljson-sem-recompilar)
5. [Criação de Novos Country Packs Internacionais](#5-criação-de-novos-country-packs-internacionais)
6. [Implementação de Novas Portas de Infraestrutura (Transporte e Cache)](#6-implementação-de-novas-portas-de-infraestrutura)
7. [Diretrizes Rigorosas de Engenharia e Checklist de Qualidade](#7-diretrizes-rigorosas-de-engenharia-e-checklist-de-qualidade)

---

## 1. Arquitetura Hexagonal DOD e Filosofia de Extensão

O BRHealth opera sob a arquitetura **Hexagonal Orientada a Dados (Hexagonal DOD)**:

```text
┌─────────────────────────────────────────────────────────────┐
│                       INBOUND PORTS                         │
│       (CLI / Python PyO3 / C-ABI FFI / Panama JNI)          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                        DOMAIN CORE                          │
│          (Apache Arrow RecordBatch, Memória Contígua,        │
│          Zero I/O, Zero Alocações Ociosas, Zero Unwraps)    │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 SERVICE PROVIDER INTERFACE (SPI)            │
│            HealthDataSourceSPI / Outbound Ports             │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  ADAPTERS & INFRASTRUCTURE                  │
│       (DatasusBlastDecoder, Tokio FTP, HTTP Streaming,      │
│            Hive-Parquet Storage, GeoArrow, NetCDF)          │
└─────────────────────────────────────────────────────────────┘
```

### Regras Arquiteturais Fundamentais:
1. **Núcleo de Domínio Puro (`domain/`)**: Nunca adicione dependências de rede, I/O de disco, chamadas de shell ou `tokio` dentro dos módulos de domínio analítico.
2. **Zero Unwraps**: O uso de `.unwrap()` ou `.expect()` é expressamente proibido em todo o código de produção (`src/`). Erros devem ser mapeados em `PortError` ou tipos de erro de domínio.
3. **Memória Contígua e Zero-Copy**: Todos os dados decodificados devem ser estruturados diretamente em buffers Apache Arrow (`RecordBatch`) alinhados a 64 bytes.

---

## 2. O Contrato SPI: `HealthDataSourceSPI`

Toda fonte de dados no BRHealth implementa a trait assíncrona `HealthDataSourceSPI`, localizada em `crates/brhealth-core/src/domain/source_spi.rs`:

```rust
#[async_trait]
pub trait HealthDataSourceSPI: Send + Sync {
    /// Retorna os metadados descritivos da fonte (ID, nome oficial, órgão, periodicidade).
    fn metadata(&self) -> SourceMetadata;

    /// Retorna o esquema canônico estrito em Apache Arrow que a fonte produz.
    fn target_schema(&self) -> Arc<Schema>;

    /// Resolve parâmetros de consulta (ano, mês, UF) em um localizador de recurso físico.
    fn resolve_locator(&self, params: &DataQueryParams) -> Result<SourceLocator, PortError>;

    /// Executa o transporte assíncrono, descompressão nativa e projeção colunar.
    async fn fetch_and_decode(
        &self,
        locator: &SourceLocator,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError>;
}
```

---

## 3. Tutorial Passo a Passo: Implementando um Novo Adaptador

Vamos construir como exemplo um adaptador para o **SIASUS - Boletim de Produção Ambulatorial Individualizado (BPAI)**.

### Passo 1: Definir os Esquemas Canônicos e Metadados

Crie o arquivo em `crates/brhealth-core/src/sources/datasus/meu_adaptador.rs`:

```rust
// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

use std::sync::Arc;
use async_trait::async_trait;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::domain::ports::PortError;
use crate::domain::schema::{canonical_columns, default_values};
use crate::domain::source_spi::{
    DataQueryParams, HealthDataSourceSPI, SourceExecutionContext, SourceLocator, SourceMetadata,
};
use crate::sources::datasus::helpers::{build_columnar_batch, ColumnBuilderSpec};

pub struct SiasusBpaiSource {
    schema: Arc<Schema>,
}

impl SiasusBpaiSource {
    pub fn new() -> Self {
        let fields = vec![
            Field::new(canonical_columns::MUNICIPALITY_CODE, DataType::Utf8, false),
            Field::new(canonical_columns::COMPETENCE_YEAR, DataType::UInt16, false),
            Field::new(canonical_columns::COMPETENCE_MONTH, DataType::UInt8, false),
            Field::new(canonical_columns::PATIENT_AGE_YEARS, DataType::UInt16, true),
            Field::new(canonical_columns::PATIENT_SEX, DataType::Utf8, true),
            Field::new(canonical_columns::PROCEDURE_CODE, DataType::Utf8, false),
            Field::new(canonical_columns::PRIMARY_DIAGNOSIS, DataType::Utf8, true),
            Field::new(canonical_columns::QUANTITY, DataType::UInt32, false),
        ];

        Self {
            schema: Arc::new(Schema::new(fields)),
        }
    }
}
```

---

### Passo 2: Implementar a Resolução de Localizadores (`resolve_locator`)

O localizador mapeia como a URL do FTP governamental é formada a partir da UF e da data:

```rust
impl HealthDataSourceSPI for SiasusBpaiSource {
    fn metadata(&self) -> SourceMetadata {
        SourceMetadata {
            id: "datasus.siasus_bpai".into(),
            display_name: "SIA-SUS - Boletim Ambulatorial Individualizado (BPAI)".into(),
            maintaining_agency: "DATASUS / Ministério da Saúde".into(),
            temporal_resolution: "Mensal (1994-2026)".into(),
            geographic_granularity: "Municipal (Código IBGE de 6 e 7 dígitos)".into(),
            fair_license: "Domínio Público Governamental / Open Data".into(),
            underlying_format: "PKWARE DCL (.dbc) / xBase DBF".into(),
        }
    }

    fn target_schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }

    fn resolve_locator(&self, params: &DataQueryParams) -> Result<SourceLocator, PortError> {
        let uf = params
            .jurisdiction_code
            .as_deref()
            .ok_or_else(|| PortError::InvalidArgument("UF é obrigatória para SIA-SUS".into()))?
            .to_uppercase();

        let month = params
            .month
            .ok_or_else(|| PortError::InvalidArgument("Mês é obrigatório para SIA-SUS".into()))?;

        // Convenção DATASUS: PA + UF + YY + MM + .dbc (ex: PABA2305.dbc)
        let yy = params.year % 100;
        let filename = format!("PA{}{:02}{:02}.dbc", uf, yy, month);
        let path = format!("/dissemin/publicos/SIASUS/200801_/Dados/{}", filename);

        Ok(SourceLocator::Ftp {
            host: "ftp.datasus.gov.br".into(),
            path,
        })
    }
```

---

### Passo 3: Decodificação Colunar Vetorizada (`fetch_and_decode`)

Aproveite o utilitário `build_columnar_batch` para decodificar registros do DBF diretamente em vetores tipados com pré-alocação precisa de capacidade:

```rust
    async fn fetch_and_decode(
        &self,
        locator: &SourceLocator,
        context: &SourceExecutionContext,
    ) -> Result<Vec<RecordBatch>, PortError> {
        // 1. Download resiliente com stream hashing
        let raw_bytes = context.transport.fetch(locator).await?;

        // 2. Descompressão Blast PKWARE DCL 100% nativa em Rust
        let dbf_bytes = context.decompressor.decompress_dbc(&raw_bytes)?;

        // 3. Parsing do cabeçalho DBF
        let dbf_reader = crate::decoders::dbf::DbfReader::new(&dbf_bytes)?;
        let num_records = dbf_reader.num_records();

        // 4. Especificações de Projeção Colunar Canônica
        let specs = vec![
            ColumnBuilderSpec::Text {
                source_column: "PA_UFMUN",
                default_value: default_values::MUNICIPALITY_UNKNOWN,
            },
            ColumnBuilderSpec::UInt16 {
                source_column: "PA_ANO",
                default_value: 0,
            },
            ColumnBuilderSpec::UInt8 {
                source_column: "PA_MES",
                default_value: 0,
            },
            ColumnBuilderSpec::UInt16 {
                source_column: "PA_IDADE",
                default_value: 0,
            },
            ColumnBuilderSpec::Text {
                source_column: "PA_SEXO",
                default_value: default_values::SEX_UNKNOWN,
            },
            ColumnBuilderSpec::Text {
                source_column: "PA_PROC_ID",
                default_value: default_values::TEXT_UNKNOWN,
            },
            ColumnBuilderSpec::Text {
                source_column: "PA_CIDPRI",
                default_value: default_values::CID10_UNKNOWN,
            },
            ColumnBuilderSpec::UInt32 {
                source_column: "PA_QTDPRO",
                default_value: 1,
            },
        ];

        // 5. Construção vetorizada direta no esquema Apache Arrow
        let batch = build_columnar_batch(
            self.schema.clone(),
            &dbf_reader,
            &specs,
            num_records,
        )?;

        Ok(vec![batch])
    }
}
```

---

### Passo 4: Registro no `CountryPack` ou `SourceRegistry`

No arquivo `crates/brhealth-core/src/sources/mod.rs`, instancie seu novo adaptador e adicione ao pacote:

```rust
pub fn create_pack_brasil() -> CountryPack {
    let mut pack = CountryPack::new("Brasil", "BRA");
    pack.register_source(Arc::new(SiasusBpaiSource::new()));
    // ... demais fontes
    pack
}
```

---

## 4. Fontes Declarativas via Manifestos YAML/JSON

Para integrar fontes externas sem a necessidade de compilar código Rust, o BRHealth fornece o carregador dinâmico `DeclarativeDataSource`.

### Exemplo de Manifesto (`fonte_declarativa.json`):

```json
{
  "id": "parceiro.atencao_domiciliar",
  "display_name": "Programa Melhor em Casa - Atendimentos",
  "maintaining_agency": "Secretaria Municipal de Saúde",
  "temporal_resolution": "Mensal",
  "geographic_granularity": "Setor Censitário",
  "fair_license": "CC-BY-4.0",
  "underlying_format": "CSV / GeoArrow",
  "url_template": "https://api.saude.gov.br/v1/atendimentos?ano={year}&mes={month}&ibge={jurisdiction}",
  "schema_fields": [
    {"name": "municipio_ibge", "type": "Utf8", "nullable": false},
    {"name": "ano_competencia", "type": "UInt16", "nullable": false},
    {"name": "codigo_cid10", "type": "Utf8", "nullable": true},
    {"name": "total_visitas", "type": "UInt32", "nullable": false}
  ]
}
```

### Carregamento Dinâmico em Rust:

```rust
use brhealth_core::sources::declarative::DeclarativeDataSource;

let json_str = std::fs::read_to_string("fonte_declarativa.json")?;
let declarative_source = DeclarativeDataSource::from_json(&json_str)?;
registry.register(Arc::new(declarative_source));
```

---

## 5. Criação de Novos Country Packs Internacionais

Você pode criar um pacote nacional completo (ex: Portugal, Colômbia, Chile) implementando uma função construtora de `CountryPack`:

```rust
use brhealth_core::domain::source_spi::CountryPack;

pub fn create_pack_portugal() -> CountryPack {
    let mut pack = CountryPack::new("Portugal", "PRT");
    // Registrar fontes do SNS (Serviço Nacional de Saúde de Portugal)
    // pack.register_source(Arc::new(SnsMortalidadeSource::new()));
    // pack.register_source(Arc::new(SnsInternamentosSource::new()));
    pack
}
```

---

## 6. Implementação de Novas Portas de Infraestrutura

Caso sua infraestrutura utilize armazenamento em nuvem (Amazon S3, Google Cloud Storage, MinIO) ou mensageria, você pode estender as portas de saída (`OutboundPorts`):

### Implementar um Novo `CachePort`:

```rust
use async_trait::async_trait;
use brhealth_core::domain::ports::{CachePort, PortError};

pub struct S3ObjectCache {
    bucket: String,
    // cliente S3
}

#[async_trait]
impl CachePort for S3ObjectCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PortError> {
        // Implementar busca no S3
        Ok(None)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), PortError> {
        // Gravar no S3
        Ok(())
    }

    async fn invalidate(&self, key: &str) -> Result<(), PortError> {
        Ok(())
    }
}
```

---

## 7. Diretrizes Rigorosas de Engenharia e Checklist de Qualidade

Antes de abrir um *Pull Request* ou submeter um novo módulo ao BRHealth, certifique-se de que seu código cumpre os 7 mandamentos de qualidade do projeto:

- [ ] **Zero Unwraps**: Nenhuma ocorrência de `.unwrap()` ou `.expect()` em `src/`. Tratamento explícito via `Result<T, PortError>`.
- [ ] **Constantes Canônicas**: Usar estritamente nomes de colunas de `domain::schema::canonical_columns` e valores sentinela de `domain::schema::default_values`.
- [ ] **Clippy Estrito**: Compilar sem nenhum aviso sob `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] **Testes de Unidade e Propriedades**: Adicionar testes unitários abrangentes com dados de borda e *property-based testing* via `proptest`.
- [ ] **Doc-tests Executáveis**: Todos os blocos de código nas docstrings devem ser compiláveis via `cargo test --doc`.
- [ ] **Cabeçalho de Direitos Autorais**: Todos os arquivos `.rs` devem conter:
  ```rust
  // Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
  // Licensed under the GNU Affero General Public License v3 (AGPLv3)
  // or a commercial license agreement directly with the author.
  ```
- [ ] **Formulação Científica**: Qualquer algoritmo ou métrica matemática nova deve ter sua fórmula documentada em LaTeX nas docstrings e manuais.
