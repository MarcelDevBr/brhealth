# Regras de Boas Práticas: SOLID, Clean Code e Idiomatic Rust

O código-fonte do BRHealth deve atingir o mais alto padrão de legibilidade, coesão, desacoplamento e segurança tipada.

---

## 1. Princípios SOLID Adaptados ao Rust e Hexagonal DOD

1. **S - Single Responsibility Principle (SRP)**:
   - Cada módulo, struct ou função deve ter apenas uma razão para mudar.
   - O cálculo do DV do IBGE não realiza I/O; o parser DBF não descompacta arquivos; a trait SPI não orquestra threads.

2. **O - Open/Closed Principle (OCP)**:
   - Aberto para extensão, fechado para modificação.
   - Novas bases de dados (nacionais ou internacionais) são adicionadas via implementações de `HealthDataSourceSPI` ou Country Packs em `sources/`, sem alterar uma única linha do `domain/registry.rs` ou do motor central.

3. **L - Liskov Substitution Principle (LSP)**:
   - Qualquer implementação de porta (ex: `TransportPort`, `DecompressorPort`) deve cumprir integralmente o contrato abstrato sem efeitos colaterais inesperados ou comportamentos que quebrem os chamadores.

4. **I - Interface Segregation Principle (ISP)**:
   - Nenhuma interface ou trait deve forçar um componente a depender de métodos que ele não utiliza.
   - Portas outbound são atômicas: `TransportPort`, `DecompressorPort`, `LocalCachePort`, `SyncStatePort`.

5. **D - Dependency Inversion Principle (DIP)**:
   - O núcleo do domínio depende exclusivamente de abstrações (traits). Os detalhes de infraestrutura (Tokio HTTP, clientes FTP, cache SQLite) dependem das abstrações de domínio e são injetados em runtime via `Arc<dyn Port>`.

---

## 2. Padrões Clean Code e Idiomatic Rust

1. **Eliminação Total de `unwrap()` e `expect()`**:
   - Em código de produção (`src/`), é expressamente proibido o uso de `.unwrap()` ou `.expect()`.
   - Utilize o operador `?` para propagar erros tipados encapsulados no enum `PortError` (ou enums de erro locais mapeados).

2. **Tipagem Forte e Evitação de "Primitive Obsession"**:
   - Evite strings livres para identificar estados ou entidades cruciais. Utilize enums estruturados (`GeographicScope`, `SourceCategory`).

3. **Imutabilidade por Padrão**:
   - Variáveis são imutáveis a menos que `mut` seja estritamente indispensável.
   - Prefira transformações funcionais (iteradores, combinadores `map`, `filter`, `and_then`) sobre loops imperativos mutáveis quando isso aumentar a clareza.

4. **Conformidade com Clippy**:
   - Todo código deve passar limpo em:
     ```bash
     cargo clippy --all-targets -- -D warnings
     ```
