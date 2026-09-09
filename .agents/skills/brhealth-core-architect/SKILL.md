---
name: brhealth-core-architect
description: >-
  Use this skill when designing, reviewing or extending the Hexagonal Data-Oriented Design
  (Hexagonal DOD), Apache Arrow columnar schemas, Zero-Copy C Data Interface FFI, or Tokio/Rayon
  concurrency pipelines in the BRHealth engine.
---

# BRHealth Core Architect Skill

Esta skill instrui o agente na arquitetura de sistemas, isolamento de camadas, aplicação estrita de SOLID/Clean Code e gerenciamento de memória do **BRHealth**.

## 1. Princípios de Hexagonal DOD & SOLID
- **Single Responsibility (SRP)**: O módulo `domain/` é estritamente puro. Nunca importar `tokio`, sockets de rede, banco de dados ou I/O de disco neste diretório.
- **Open/Closed (OCP)**: Fontes e formatos novos são adicionados através da trait `HealthDataSourceSPI` sem modificar o motor central de consultas.
- **Interface Segregation (ISP)**: Portas outbound são granulares e atômicas (`TransportPort`, `DecompressorPort`, `LocalCachePort`, `SyncStatePort`).
- **Dependency Inversion (DIP)**: O domínio depende unicamente de traits. Adaptadores concretos (`infrastructure/`, `decoders/`) são injetados em tempo de execução.
- **Liskov Substitution (LSP)**: Qualquer implementação de porta deve satisfazer o contrato sem lançar panics inesperados.

## 2. Clean Code e Tolerância Zero a Unwraps
- Em código de produção (`src/`), `.unwrap()` e `.expect()` são **estritamente proibidos**.
- Erros são sempre representados via `thiserror` no enum `PortError` e propagados via operador `?`.
- Variáveis são imutáveis por padrão.

## 3. Padrão Zero-Copy Arrow FFI
- As estruturas internas de transferência são sempre `arrow::record_batch::RecordBatch` alinhados a 64 bytes.
- A comunicação com Python (`PyO3`), C++20 ou JVM (Panama FFM) utiliza a Arrow C Data Interface:
  ```rust
  let (ffi_array, ffi_schema) = batch.into_ffi_ptrs();
  unsafe {
      *out_array = ffi_array;
      *out_schema = ffi_schema;
  }
  ```

## 4. Concorrência Segura: Tokio vs. Rayon
- **I/O Bound (Rede, Download, FTP, HTTP)**: Executar no runtime assíncrono `tokio`.
- **CPU Bound (Descompressão Blast DCL, parsing DBF, cálculo de DV IBGE)**: Executar no pool de threads de computação paralela `rayon`.
- **Ponte**: Quando uma tarefa assíncrona precisar de decodificação massiva de CPU, use `tokio::task::spawn_blocking` despachando para `rayon`.

## 5. Checklist de Verificação de Arquitetura
1. [ ] O novo módulo manteve o `domain/` 100% puro e livre de I/O?
2. [ ] Todas as funções públicas possuem docstrings com doc-tests executáveis?
3. [ ] Todos os caminhos de erro retornam `Result<T, PortError>`?
4. [ ] O código passa sem avisos em `cargo clippy -- -D warnings`?
