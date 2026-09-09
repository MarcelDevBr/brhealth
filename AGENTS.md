# Diretrizes Arquiteturais e de Engenharia - BRHealth

Este repositório implementa o **BRHealth**, um motor analítico colunar de alta performance para dados de saúde coletiva, bioestatística e determinantes sociais, desenvolvido sob a arquitetura **Hexagonal Orientada a Dados (Hexagonal DOD)**.

---

## 1. Princípios de Engenharia Obrigatórios

1. **Separação Rígida de Camadas (Hexagonal DOD)**:
   - **`domain/` (Núcleo Puro)**: Não pode conter dependências de I/O, rede, banco de dados ou `tokio`. O domínio opera exclusivamente sobre estruturas de memória contígua (Apache Arrow), transformações matemáticas, validações ontológicas e interfaces abstratas (traits/ports).
   - **`ports/`**: Define contratos estritos de entrada (`InboundPorts`) e saída (`OutboundPorts` / `HealthDataSourceSPI`).
   - **`decoders/` e `infrastructure/`**: Implementam os adaptadores concretos (descompressor Blast PKWARE DCL, parsers DBF/GeoArrow, clientes assíncronos HTTP/FTP, cache Hive-Parquet).

2. **Memória Contígua e Zero-Copy (Apache Arrow)**:
   - Toda estrutura tabular interna reside em `RecordBatch` alinhado a 64 bytes conforme o padrão Apache Arrow.
   - Qualquer travessia de linguagem (Rust $\to$ Python, C++20, Java 21+) deve ser realizada estritamente via **Arrow C Data Interface** ou **DLPack**, sendo expressamente proibidas cópias intermediárias de buffers ou serializações de/para JSON, CSV ou Protobuf em pipelines de alto volume.

3. **Independência de Binários Externos**:
   - Todo decodificador (notadamente o algoritmo PKWARE DCL / Blast do DATASUS para arquivos `.dbc`) deve ser implementado de forma 100% nativa e segura em Rust, sem invocação de comandos de shell (`dbc2dbf`) ou bibliotecas dinâmicas externas legadas em C.

4. **Reprodutibilidade Científica e Padrões FAIR (W3C PROV-O)**:
   - Toda extração e agregação analítica deve ser acompanhada de geração de manifesto de linhagem criptográfica com hashes **SHA-256** dos dados brutos consumidos, carimbos temporais UTC e versionamento de ontologias.

5. **SOLID, Clean Code e Rigor Tecnológico**:
   - **Single Responsibility**: Funções e módulos concisos e desacoplados.
   - **Open/Closed**: Extensibilidade contínua via contratos SPI sem modificação do núcleo do domínio.
   - **Liskov Substitution & Interface Segregation**: Portas outbound atômicas e polimorfismo rigorosamente verificado.
   - **Dependency Inversion**: O domínio depende exclusivamente de abstrações, com injeção de dependência via traits assíncronas.
   - **Zero Unwraps**: Expressamente proibido o uso de `.unwrap()` ou `.expect()` em código de produção (`src/`). Todo tratamento de falha deve ser tipado e propagado via `Result<T, PortError>`.
   - **Clippy Estrito**: Todo código deve compilar sem avisos sob `cargo clippy -- -D warnings`.

6. **Pirâmide de Testes e Cobertura Total**:
   - Meta de **100% de cobertura** no domínio analítico, algoritmos de transformação matemática e decodificadores binários.
   - Todo módulo público deve conter: testes unitários exaustivos, testes baseados em propriedades (*Property-Based Testing* via `proptest`), doc-tests executáveis (`cargo test --doc`) e testes de robustez/fuzzing para formatos binários.

7. **Rigor Acadêmico Máximo e Documentação Formal**:
   - Algoritmos de cálculo, taxas epidemiológicas e transformações territoriais devem ter suas formulações matemáticas formalmente documentadas em **LaTeX** nas docstrings.
   - Validações contra tabelas e dados canônicos oficiais (IBGE, Ministério da Saúde, OMS).

8. **Direitos Autorais e Licenciamento**:
   - O projeto é licenciado sob a **GNU Affero General Public License v3 (AGPLv3)** com modelo de duplo licenciamento comercial exclusivo do criador.
   - O aviso de direitos autorais perpétuo deve ser rigorosamente mantido em todos os cabeçalhos de arquivos e documentações:
     `Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.`
