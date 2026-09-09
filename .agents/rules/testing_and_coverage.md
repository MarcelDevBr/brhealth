# Regras de Pirâmide de Testes e Cobertura de Código

Para garantir a confiabilidade necessária a um software científico e biomédico de missão crítica, o BRHealth adota uma política de testes estrita:

---

## 1. Níveis da Pirâmide de Testes

1. **Testes de Unidade (`#[test]`)**:
   - Todo algoritmo matemático, validação e manipulador de buffer deve ter testes unitários cobrindo casos nominais, casos de borda (*edge cases*) e casos patológicos.
   - Devem executar em milissegundos sem dependências de rede ou disco real.

2. **Testes Baseados em Propriedades (*Property-Based Testing* com `proptest`)**:
   - Algoritmos como o cálculo do DV do IBGE (Módulo 10) e descompressão de bits devem ser testados contra centenas de milhares de entradas pseudoaleatórias geradas automaticamente, validando invariantes matemáticas que testes manuais não alcançam.

3. **Doc-Tests Executáveis (`/// ```rust`)**:
   - Toda função, trait ou struct pública exposta pelo crate deve conter ao menos um exemplo executável em sua docstring.
   - Os doc-tests são validados continuamente via `cargo test --doc`.

4. **Testes de Integração (`tests/`)**:
   - Testam a orquestração entre adaptadores, registros SPI e schemas Arrow.
   - Podem utilizar dados sintéticos ou fixtures de teste compactas.

5. **Testes de Robustez e Fuzzing**:
   - Decodificadores binários (PKWARE DCL `.dbc`, DBF) devem ser testados contra fluxos corrompidos, truncados ou malformados, garantindo que o motor retorne `PortError::DecompressionError` sem disparar panics ou aborts de processo.

---

## 2. Metas de Cobertura e Critérios de Aceitação

- **Domínio e Transformações (`src/domain/`)**: **100% de cobertura de linhas e branches**.
- **Decodificadores (`src/decoders/`)**: Cobertura superior a 95%, com todos os caminhos de erro cobertos.
- **Proibição de Bypass**: Nenhum teste pode ser desabilitado via `#[ignore]` sem justificativa formal e prazo explícito de resolução.
