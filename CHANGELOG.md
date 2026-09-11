# Histórico de Mudanças — BRHealth

Todas as alterações notáveis no projeto **BRHealth** serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [0.1.0] - 2026-09-11

### Adicionado
- Versão inicial do motor analítico colunar **BRHealth** em Rust 2024 sob arquitetura Hexagonal DOD.
- Suporte a 26 fontes de dados de saúde e determinantes sociais (DATASUS SIM, SINASC, SIH-SUS, SIAB/SISAB, CadÚnico, Censo IBGE, Copernicus ERA5).
- Descompressor nativo 100% Rust PKWARE Blast para arquivos `.dbc` do DATASUS.
- Decodificador colunar DBF para Apache Arrow Zero-Copy (`RecordBatch`).
- Mapeamento e harmonização territorial de códigos municipais do IBGE (Luhn Módulo 10 DV) de 1970 a 2026.
- Classificação analítica de Causas de Internações Sensíveis à Atenção Primária (CSAP — Portaria MS/SAS nº 221/2008).
- Cálculo de Anos Potenciais de Vida Perdidos (APVP / YLL) e ROI da Atenção Primária.
- Suporte a indexação espacial discreta Uber H3 e Google S2 Geometry.
- Geração de manifestos de proveniência auditáveis W3C PROV-O com hash SHA-256.
- Bindings Python via PyO3 e exportação Zero-Copy para Polars, PyArrow, Pandas e PyTorch via DLPack.
- **Normalização Transparente de IDs de Fonte**: O `SourceRegistry` aceita tanto notação com ponto (`datasus.sih`) quanto com underscore (`datasus_sih`), facilitando a migração e reduzindo erros de primeiro uso.
- **Mensagens de Erro Enriquecidas com Sugestões**: `SourceRegistry::get` calcula distância Levenshtein e sugere a fonte mais próxima quando um ID inválido é informado (ex: *"Você quis dizer 'datasus.sih'?"*), além de listar todas as fontes registradas.
- **Exceções Tipadas em Python via PyO3**: Introduzidas as exceções `BRHealthError`, `SourceNotFoundError`, `TransportError` e `ValidationError` no pacote Python `brhealth`.
- **Diagnóstico de Ambiente Python**: Adicionada a função `brhealth.check_environment()` para verificar a presença de `pyarrow`, `polars`, `pandas` e `torch`.
- **Captura Segura de Panics FFI**: Rotina `safe_catch_panic` integrada para capturar panics Rust em chamadas FFI e convertê-los em exceções Python sem abortar o processo Python/Jupyter/Colab.
- **Documentação de Governança LGPD**: Criado o documento `docs/LGPD_GOVERNANCE.md` com salvaguardas e diretrizes de proteção de dados sensíveis de saúde.
- **Arquivos de Governança de Projeto**: Adicionados `SECURITY.md`, `CONTRIBUTING.md` e `.github/dependabot.yml`.
- **CI/CD Multi-SO & Sanity**: Matriz de testes Linux, macOS e Windows, verificação de símbolos empacotados e auditoria com `cargo audit`.

### Corrigido
- **Perfil de Release FFI**: Removida a flag `panic = "abort"` do `Cargo.toml` raiz para permitir desbobinamento de pilha (*unwinding*) em bindings FFI (Python/Java).
- **Docstrings e Exemplos CLI/README**: Atualizados os comandos de exemplo para utilizar a notação canônica de fontes com ponto (`datasus.sih`).
