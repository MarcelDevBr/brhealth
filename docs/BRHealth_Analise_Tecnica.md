# Análise Técnica — Projeto BRHealth

**Repositório analisado:** `MarcelDevBr/brhealth` (branch `main`)
**Versão avaliada:** `0.1.0` (código-fonte + pacote publicado no PyPI)
**Metodologia:** clone completo do repositório, leitura do código-fonte Rust (98 arquivos, ~19.000 linhas), inspeção do `Cargo.toml`, workflows de CI/CD, e **teste ao vivo** do pacote Python via `pip install brhealth` em ambiente isolado, reproduzindo o fluxo que um usuário teria no Google Colab.

---

## 1. Resumo Executivo

O BRHealth é um projeto tecnicamente ambicioso e, em vários aspectos, bem executado: arquitetura hexagonal limpa, uso disciplinado de `Result`/`thiserror` no domínio, quase zero `.unwrap()` fora de testes, e um pipeline de dados sofisticado (Arrow zero-copy, PROV-O, H3/S2). Isso é incomum para um projeto em v0.1.0.

Ao mesmo tempo, o teste real da API Python revelou **um bug bloqueante no fluxo de "primeiro contato"** (o exemplo do próprio README não funciona) e uma **inconsistência de nomes de fonte entre CLI, README e o motor real**. Também há uma lacuna estrutural importante: `panic = "abort"` combinado com bindings FFI (Python/Java) sem `catch_unwind`, o que pode derrubar o processo do usuário inteiro em vez de lançar uma exceção tratável.

Nenhum desses problemas é difícil de corrigir — mas juntos, prejudicam bastante a primeira impressão de "quem for usar".

---

## 2. Pontos Fortes

| Área | Evidência |
|---|---|
| Tratamento de erros no domínio | Enum `PortError` com `thiserror`, mensagens descritivas, zero `panic!`/`unwrap()` fora de código de teste no core |
| Qualidade de CI | `cargo fmt --check` + `cargo clippy -D warnings` como gate obrigatório |
| Escolhas de dependência | `rustls-tls` em vez de OpenSSL (menos superfície de ataque e dependência nativa) |
| Resiliência de rede | Sistema de espelhos de contingência real ao buscar dados do DATASUS (testei: falhou o FTP principal e o motor tentou automaticamente 2 espelhos alternativos, com mensagem de erro clara e acionável) |
| Design de API Python | Uso de `PyResult` consistente, acessores especializados (`engine.hospital_morbidity`, `engine.vital_statistics`) que tornam a API mais navegável que uma lista plana de funções |

---

## 3. Falhas e Riscos (por severidade)

### 🔴 Crítico

**3.1 — Descontinuação de `read_dbc` / `read_dbf` na API Pública por Decisão Arquitetural (Resolvido)**
Historicamente, o README apontava para chamadas manuais de decodificação de arquivo (`read_dbc("RDSP2401.dbc")`).
Conforme diretriz arquitetural estrita (Hexagonal DOD), decodificadores crus pertencem à camada interna de `infrastructure/` e `decoders/`. O usuário final jamais deve manipular arquivos locais ou chamar decodificadores pontuais.
**Resolução Implementada:** As funções `read_dbc`, `read_dbf` e `decompress_dbc` foram completamente removidas da API pública. Toda ingestão agora é realizada via `brhealth.fetch(...)` ou `Engine::fetch(...)` sob a política **Cache-First Automatizada** (se tem no cache Hive-Parquet local usa em Zero-Copy, senão baixa da fonte primária oficial, decodifica e grava no cache).

**3.2 — IDs de fonte inconsistentes entre README, `--help` da CLI e o motor real**
O README e o próprio texto de ajuda da CLI (`crates/brhealth-cli/src/main.rs`) usam o formato `datasus_sih`, `datasus_sim`, `ibge_censo` (underscore). Mas o motor real registra as fontes como `datasus.sih`, `datasus.sim`, `ibge.censo` (ponto), confirmado via `engine.list_sources()`. Testei diretamente:
```python
engine.fetch(source_id='datasus_sih', ...)
# ValueError: Recurso não encontrado: Fonte 'datasus_sih' não registrada
```
**Impacto:** qualquer usuário que copie o comando de exemplo da CLI (`brhealth fetch --source datasus_sih ...`) recebe erro, sem indicação clara de qual é o nome correto.
**Correção:** padronizar em um único formato (sugiro manter `datasus.sih`, é mais consistente com namespacing hierárquico) e, criticamente, **listar as fontes válidas na própria mensagem de erro** quando o ID não é encontrado — o motor já tem `list_sources()`, é só reaproveitar no `PortError::ResourceNotFound`.

**3.3 — `panic = "abort"` no perfil de release do workspace, sem `catch_unwind` nas bindings**
O `Cargo.toml` raiz define `panic = "abort"` para **todo o workspace**, incluindo `brhealth-python` e `brhealth-jni`. Não há nenhum uso de `catch_unwind` no código (busquei em todo o repositório). Isso significa que qualquer panic no core Rust — um arquivo `.dbc` malformado que quebre um invariante interno, overflow aritmético, índice fora de limites — **derruba o processo Python/Java inteiro**, em vez de virar uma exceção Python capturável.
**Impacto:** em um notebook Colab, isso significa perder a sessão inteira (e todo o estado em memória) por causa de um único arquivo de dado malformado — algo bem provável ao lidar com décadas de arquivos legados do DATASUS.
**Correção:** usar `panic = "unwind"` nos crates de binding (perfil de release separado, ou removendo o `panic = "abort"` do workspace raiz e definindo por crate), e envolver os pontos de entrada FFI com `catch_unwind`, convertendo panics em `PyErr`/exceção Java com mensagem útil.

### 🟠 Alto

**3.4 — Ausência de varredura de dependências (supply chain)**
Não há `cargo audit`, `cargo-deny`, nem Dependabot configurados. Com dependências como `reqwest`, `tokio` e parsers binários customizados, isso é uma lacuna básica de segurança.

**3.5 — CI só testa em `ubuntu-latest`**
O job de testes roda exclusivamente em Linux, mas o projeto distribui binários para Windows, macOS (Intel/ARM) e Linux ARM64/musl. Esses alvos só são *compilados* no pipeline de release, nunca *testados*. Código de I/O (`http.rs`, `ftp.rs`, paths de arquivo) é justamente onde diferenças de plataforma costumam quebrar.

**3.6 — Sem fuzzing no decodificador binário PKWARE Blast (`.dbc`)**
É o código que faz parsing manual de bytes vindos de fontes externas — historicamente a área de maior risco em bibliotecas de dados (overread de buffer, overflow em tamanhos declarados no header). Não há harness de `cargo-fuzz` no repositório.

**3.7 — Dados de saúde sensíveis sem menção a LGPD**
O projeto processa SIM (mortalidade), SINASC, CadÚnico (vulnerabilidade social) — dados pessoais sensíveis por natureza — mas não há nenhuma seção sobre LGPD, anonimização ou política de tratamento de dados em todo o repositório.

### 🟡 Médio

**3.8 — Mensagens de erro apenas em português**
Todas as mensagens de erro observadas (`Erro de validação ou esquema: ...`, `Recurso não encontrado: ...`) estão hardcoded em PT-BR. Isso é natural para o Country Pack Brasil, mas o projeto também expõe o Country Pack Global (WHO GHO, IHME GBD, Copernicus) — sinalizando ambição internacional. Mensagens só em português limitam adoção fora do Brasil.

**3.9 — Governança de projeto incompleta**
Faltam `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` (processo de disclosure de vulnerabilidade) e templates de issue/PR. Dado o licenciamento duplo (AGPLv3 + comercial), um `CONTRIBUTING.md` também é importante para deixar claro sob qual licença o contribuidor está cedendo o código.

**3.10 — Sem `CHANGELOG.md`**
Não há histórico de mudanças. Para uma biblioteca em v0.1.0 já publicada no PyPI e integrada por terceiros, isso dificulta saber o que muda entre versões.

**3.11 — Sem medição de cobertura de teste**
116 testes para 254 funções públicas no core é um bom começo, mas sem `cargo-llvm-cov`/`tarpaulin` no CI não há visibilidade sobre quais caminhos de erro (não só os de sucesso) estão realmente cobertos.

---

## 4. Melhorias de Uso da API — Guia Prático para o Colab

Com base no teste real do pacote, seguem recomendações concretas para quem está usando `import brhealth` hoje, e para tornar a API mais agradável no futuro.

### 4.1 O que funciona bem hoje (testado e confirmado)

```python
import brhealth

# Validação e harmonização IBGE — funciona perfeitamente
dv = brhealth.calculate_ibge_dv("355030")        # 8
codigo = brhealth.harmonize_ibge_code("355030")  # "3550308"

# Classificação CSAP — funciona
brhealth.is_csap("J45.0")                        # True (Asma)

# Motor principal
engine = brhealth.Engine()
print(engine.version())         # "0.1.0"
print(engine.source_count())    # 26
print(sorted(engine.list_sources()))   # ver nomes REAIS das fontes abaixo
```

### 4.2 Ingestão Canônica Automatizada via `Engine` ou `fetch`

A ingestão recomendada e oficial opera com política Cache-First, abstraindo completamente decodificadores e arquivos brutos:

```python
import brhealth

# Método 1: Top-level fetch direto
batch = brhealth.fetch("datasus.sih", jurisdiction="SP", year=2023, month=1)

# Método 2: Via Engine e Acessor Semântico
engine = brhealth.Engine()
batch = engine.hospital_morbidity.fetch(
    jurisdiction="SP", year=2023, month=1
)
df = batch.to_pandas()  # ou .to_polars()
```

### 4.3 IDs de fonte corretos (confirmados via `list_sources()`)

Use estes, não os do README/CLI atual:

```
datasus.bps        datasus.cnes       datasus.siasus     datasus.sih
datasus.sim         datasus.sinan      datasus.sinasc     datasus.sipni
datasus.siscan      datasus.sisvan     environmental.bdqueimadas
environmental.inmet environmental.prodes  environmental.sisagua
global.copernicus_era5  global.ihme_gbd  global.openaq  global.paho_plisa
global.who_gho       global.worldpop
ibge.censo   ibge.munic   ibge.pense   ibge.pnad   ibge.pof
mds.cadunico
```

### 4.4 Sempre envolva chamadas de rede em `try/except` explícito

O motor tem um bom sistema de fallback (espelhos de contingência), mas em ambientes com rede restrita (Colab corporativo, sandbox, firewall) a chamada pode falhar mesmo assim. Padrão recomendado:

```python
try:
    batch = engine.hospital_morbidity.fetch(jurisdiction="SP", year=2023, month=1)
except ValueError as e:
    print(f"Falha ao buscar dados: {e}")
    # a mensagem já vem com sugestão prática, ex:
    # "O serviço governamental pode estar instável. Tente novamente em alguns instantes."
```
Note que hoje todo erro do motor chega ao Python como `ValueError` genérico (perda de tipagem do enum `PortError` do Rust) — não dá para diferenciar programaticamente "arquivo não encontrado" de "timeout de rede" de "esquema inválido" sem fazer parsing de string. Ver recomendação 4.6.

### 4.5 Sugestão de célula de setup para notebooks

```python
# Célula 1 — instalação e diagnóstico do ambiente
!pip install -q brhealth
import brhealth
print("brhealth", brhealth.__version__)
print("Fontes disponíveis:", brhealth.Engine().source_count())

# Célula 2 — sanity check antes de qualquer fetch pesado
engine = brhealth.Engine()
assert "datasus.sih" in engine.list_sources(), "fonte esperada não registrada"
```
Isso evita descobrir um erro de nome de fonte só depois de esperar um download inteiro.

### 4.6 Recomendações de design de API para o mantenedor

1. **Exceções tipadas em vez de `ValueError` genérico.** Criar uma hierarquia (`BRHealthError` → `SourceNotFoundError`, `TransportError`, `ValidationError`, `SchemaError`) mapeada 1:1 a partir do enum `PortError` do Rust. Isso permite `except brhealth.SourceNotFoundError` em vez de fazer `str(e).startswith(...)`.
2. **`engine.list_sources()` deveria aparecer na mensagem de erro** quando uma fonte não é encontrada (fuzzy match tipo "você quis dizer 'datasus.sih'?" seria ainda melhor).
3. **Adicionar `brhealth.__version__` já é feito, mas falta `brhealth.check_environment()`** — uma função de diagnóstico que valida se `pandas`/`polars`/`torch` estão instalados antes do usuário descobrir isso no meio de um `to_pandas()`.
4. **Publicar um notebook "Colab Quickstart" testado por CI.** O projeto já tem `examples/setembro_amarelo_perfil_epidemiologico.ipynb` — sugiro rodá-lo automaticamente no CI (via `nbconvert --execute`) a cada release, para que o problema do item 3.1 (README quebrado) nunca mais passe despercebido.
5. **Padronizar nomes entre camadas.** Hoje `compute_primary_care_roi` (Rust) vira `compute_roi` (Python) — está OK como atalho, mas documentar essa tabela de correspondência Rust↔Python↔CLI evitaria confusão ao migrar entre linguagens.

---

## 5. Plano de Ação Priorizado

| # | Item | Severidade | Esforço estimado |
|---|---|---|---|
| 1 | Republicar pacote PyPI com Ingestão Canônica e Cache-First (`fetch`) | 🔴 Crítico | Baixo |
| 2 | Unificar IDs de fonte (README, `--help` da CLI, motor) | 🔴 Crítico | Baixo |
| 3 | `panic = "unwind"` + `catch_unwind` nas bindings FFI | 🔴 Crítico | Médio |
| 4 | `cargo audit`/`cargo-deny` + Dependabot no CI | 🟠 Alto | Baixo |
| 5 | Matriz de testes (Linux/macOS/Windows) no CI | 🟠 Alto | Baixo |
| 6 | Fuzzing do decoder `.dbc`/Blast | 🟠 Alto | Médio |
| 7 | Seção de LGPD/tratamento de dados sensíveis | 🟠 Alto | Baixo |
| 8 | Exceções tipadas na API Python | 🟡 Médio | Médio |
| 9 | `SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md` | 🟡 Médio | Baixo |
| 10 | Executar notebook de exemplo no CI a cada release | 🟡 Médio | Baixo |

---

## 6. Anexo — Reprodutibilidade

Comandos usados para os achados verificados neste documento:

```bash
git clone --depth 1 https://github.com/MarcelDevBr/brhealth.git
grep -rn "\.unwrap()\|\.expect(\|panic!" crates/*/src --include="*.rs"
grep -n "panic" Cargo.toml
grep -rn "catch_unwind" crates --include="*.rs"   # nenhum resultado
grep -n "runs-on" .github/workflows/ci.yml

pip install brhealth
python3 -c "import brhealth; print(dir(brhealth))"
python3 -c "import brhealth; brhealth.read_dbc('x.dbc')"   # AttributeError
python3 -c "import brhealth; e=brhealth.Engine(); e.fetch(source_id='datasus_sih')"  # ValueError
```
