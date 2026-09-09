# Regras de Arquitetura: Hexagonal Data-Oriented Design (Hexagonal DOD)

Ao desenvolver novos módulos, portas, adaptadores ou fontes de dados no BRHealth, as seguintes regras são mandatórias:

1. **Isolamento do Núcleo de Domínio (`crates/brhealth-core/src/domain`)**:
   - Zero dependências de rede (`reqwest`, `tokio::net`, etc.).
   - Zero dependências de sistema de arquivos direto (`std::fs`, caminhos rígidos).
   - O domínio expressa transformações de dados sobre vetores Apache Arrow (`RecordBatch`, `ArrayRef`), schemas canônicos e contratos abstratos.

2. **Padrão HealthDataSourceSPI para Fontes**:
   - Nenhuma regra específica de um país ou fonte deve ser codificada no motor principal de consulta.
   - Cada fonte deve implementar a trait assíncrona `HealthDataSourceSPI`, retornando metadados descritivos, schema canônico de destino e método `fetch_and_decode`.
   - Fontes pertencem a Country Packs modulares (`pack_br`, `pack_global`, etc.).

3. **Portas e Adaptadores**:
   - As portas de saída (`TransportPort`, `DecompressorPort`, `TabularDecoderPort`, `LocalCachePort`, `SyncStatePort`) devem ser interfaces desacopladas.
   - A camada `infrastructure/` implementa o I/O assíncrono com Tokio, cache Hive-Parquet e persistência SQLite.
