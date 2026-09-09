---
name: scientific-software-quality
description: >-
  Use this skill when auditing test coverage, implementing property-based tests (proptest),
  fuzz testing decoders, benchmarking with Criterion, or enforcing SOLID, Clean Code and strict
  Clippy guidelines in BRHealth.
---

# Scientific Software Quality Skill

Esta skill fornece os procedimentos e comandos para assegurar cobertura total de testes, testes baseados em propriedades, benchmarking e conformidade estrita com padrões de engenharia de software científico.

## 1. Comandos de Validação de Qualidade

### Execução de Testes Unitários e Doc-Tests
```bash
cargo test -p brhealth-core
cargo test -p brhealth-core --doc
```

### Análise Estrita com Clippy
```bash
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
```

### Auditoria de Cobertura de Código (via cargo-tarpaulin ou cargo-llvm-cov)
```bash
# Execução com cargo-tarpaulin
cargo tarpaulin -p brhealth-core --out Html --output-dir target/coverage/
```

## 2. Padrões de Property-Based Testing com `proptest`
Utilize `proptest` para validar invariantes matemáticas contra milhares de casos gerados aleatoriamente:
```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_ibge_dv_invariants(code in "[0-9]{6}") {
            let dv = calculate_ibge_dv(&code);
            prop_assert!(dv.is_ok());
            let dv_val = dv.unwrap();
            prop_assert!(dv_val < 10);
        }
    }
}
```

## 3. Checklist de Rigor Científico e Clean Code
1. [ ] A função possui documentação formal em LaTeX para equações matemáticas?
2. [ ] Todas as chamadas a funções que podem falhar utilizam `Result<T, PortError>` sem `.unwrap()`?
3. [ ] O código cumpre os 5 princípios SOLID?
4. [ ] Foram incluídos doc-tests executáveis demonstrando o uso correto da API?
5. [ ] Todos os testes de unidade e de propriedades passaram com 100% de sucesso?
