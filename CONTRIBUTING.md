# Guia de Contribuição — BRHealth

Agradecemos o seu interesse em contribuir com o **BRHealth**, o motor analítico colunar de alta performance para saúde coletiva e bioestatística!

---

## 1. Diretrizes de Desenvolvimento e Arquitetura

Ao submeter código para o repositório, você deve seguir estritamente as diretrizes contidas em [`AGENTS.md`](file:///home/marcel/Desenvolvimento/Projetos/brhealth/AGENTS.md):

1. **Arquitetura Hexagonal DOD**:
   - O repositório é organizado em `domain/` (puro, sem I/O ou tokio), `ports/`, `decoders/` e `infrastructure/`.
   - Mantenha o domínio desacoplado de detalhes de transporte ou banco de dados.

2. **Qualidade e Estilo de Código**:
   - **Zero `.unwrap()` ou `.expect()`** em código de produção (`src/`). Todo tratamento de erro deve ser tipado e propagado via `Result<T, PortError>`.
   - O código deve compilar sem avisos com Clippy estrito:
     ```bash
     cargo clippy --workspace --all-targets -- -D warnings
     ```
   - Formatação padrão via `cargo fmt`:
     ```bash
     cargo fmt --all -- --check
     ```

3. **Cobertura de Testes**:
   - Todo novo recurso ou correção deve ser acompanhado de testes unitários, testes de integração ou *property-based testing* (`proptest`).

---

## 2. Modelo de Licenciamento

O BRHealth é distribuído sob a licença **GNU Affero General Public License v3 (AGPLv3)** com modelo de **Duplo Licenciamento Comercial** mantido pelo criador (`MarcelDevBr`).

Ao enviar um Pull Request (PR), você concorda que:
- Sua contribuição será licenciada sob a licença AGPLv3.
- O aviso de copyright perpétuo deve ser mantido nos cabeçalhos dos arquivos:
  ```rust
  // Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
  // Licensed under the GNU Affero General Public License v3 (AGPLv3)
  // or a commercial license agreement directly with the author.
  ```

---

## 3. Fluxo de Trabalho de PRs

1. Faça um *fork* do repositório e crie uma *branch* descritiva (ex: `feature/suporte-sinan-dengue` ou `fix/read-dbc-pypi`).
2. Garanta que a suíte completa de testes passe localmente:
   ```bash
   cargo test --workspace --all-targets
   ```
3. Abra um Pull Request detalhando a motivação da mudança, testes executados e referências às Issues resolvidas.
