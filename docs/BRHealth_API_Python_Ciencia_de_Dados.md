# BRHealth — API Python para Ciência de Dados: Análise e Roteiro de Melhorias

**Repositório:** `MarcelDevBr/brhealth` · **Foco:** experiência da API Python (`import brhealth`) para análise e ciência de dados em saúde coletiva.  
**Metodologia:** auditoria estrita de `crates/brhealth-python/src/lib.rs`, testes comparativos entre a versão instalada via `pip install brhealth` e a árvore do `main`, e avaliação de interoperabilidade com o ecossistema colunar analítico (Apache Arrow, Polars, DuckDB, Pandas e PyTorch).

---

## 1. O que já está bem pensado para ciência de dados

O design de interoperabilidade de dados do BRHealth é **incomumente sofisticado** para um projeto de saúde pública:

- **Protocolo Arrow PyCapsule** (`__arrow_c_array__`, `__arrow_c_schema__`): Permite Zero-Copy real para qualquer biblioteca que suporte o padrão Apache Arrow (PyArrow, Polars, DuckDB, DataFusion), eliminando serializações intermediárias.
- **Suporte a DLPack** (`__dlpack__`, `__dlpack_device__`): Permite passar tensores de microdados de saúde diretamente para PyTorch sem cópia de memória.
- **Conversores com Fallback em Cascata** (`to_pyarrow()`, `to_polars()`, `to_pandas()`, `to_dict()`): Flexibilidade total sem forçar dependências rígidas na instalação básica.
- **Renderização Rica em HTML (`_repr_html_`)**: Renderização formatada e legível dos esquemas de dados em células interativas do Jupyter Notebook e Google Colab.
- **Séries Temporais Multi-Ano e Multi-Jurisdição**: Suporte a `years=[2020, 2021, 2022]` e `jurisdictions=["SP", "RJ", "MG"]` com concatenação colunar automática via `arrow::compute::concat_batches`.
- **Manifesto FAIR/PROV-O Exportável** (`export_fair_manifest`): Geração de linhagem criptográfica (SHA-256 e W3C PROV-O) para reprodutibilidade científica.

---

## 2. Diagnóstico Central: Defasagem entre PyPI e Código-Fonte

Na versão inicial `0.1.0` distribuída via PyPI, mais de 15 símbolos públicos documentados não estavam presentes no binário compilado.

| Categoria | Presente no código-fonte (`main`) | Presente no PyPI (`0.1.0` inicial) | Status Atual |
|---|---|---|---|
| Decodificação Nativa | Descontinuado da API pública (abstraído internamente por `fetch`) | ❌ | ✅ Ingestão automatizada com Cache-First |
| Harmonização IBGE | `validate_ibge_code`, `reconcile_historical_ibge_code` | ❌ | ✅ Implementado |
| Geoespacial Analítico | `h3_to_latlng`, `h3_index_to_coord`, `h3_grid_disk`, `s2_cell_to_coord` | ❌ | ✅ Implementado |
| Bioestatística | `compute_batch_apvp`, `compute_age_standardized_mortality_rate` | ❌ | ✅ Implementado com `allow_threads` |
| CSAP & Ontologias | `csap_group_name`, `icd10_chapter`, `validate_biological_consistency` | ❌ | ✅ Implementado |
| Ingestão Top-Level | `fetch()` no nível do módulo (atalho sem instanciar `Engine`) | ❌ | ✅ Implementado com `allow_threads` |
| Acessores Semânticos | `DemographicsAccessor`, `AmbulatoryAccessor`, `EnvironmentalAccessor`, `SocialAccessor` | ❌ | ✅ Implementado |
| `RecordBatchWrapper` | `_repr_html_`, `shape`, `head()`, `tail()`, `to_dict()`, `__getitem__` | ❌ | ✅ Implementado com Zero-Copy |
| Tipagem Estática | `brhealth.pyi` + `py.typed` (PEP 561) | ❌ | ✅ Criado e empacotado no Maturin |

**Ação mandatória:** Lançamento da release **`0.2.0`** no PyPI a partir do `main` para disponibilizar esses recursos aos usuários finais.

---

## 3. Fricções de Ergonomia e Resoluções Técnicas

### 3.1 Liberação do GIL (`py.allow_threads`)
- **Problema:** Chamadas de I/O de rede e descompressão Blast pesada bloqueavam a thread Python com o GIL retido, travando o kernel do Jupyter/Colab e impossibilitando o cancelamento com `Ctrl+C`.
- **Resolução Implementada:** Envolvimento de todas as operações de I/O, download, descompressão e chamadas de pipelines assíncronos Tokio no `Engine::fetch` e `brhealth.fetch` com `py.allow_threads(|| { ... })`.

### 3.2 Tipagem Estática e Autocomplete (PEP 561)
- **Problema:** Usuários de IDEs (VS Code, PyCharm) e notebooks não recebiam autocompletar nem checagem de tipos.
- **Resolução Implementada:**
  - Criação do stub completo `brhealth.pyi` com documentação tipada das classes, funções e parâmetros.
  - Adição do marcador `py.typed`.
  - Configuração no `pyproject.toml` sob `[tool.maturin]` (`include = ["brhealth.pyi", "py.typed"]`) para empacotamento correto dentro do wheel.

### 3.3 Ergonomia de Coleção no `RecordBatchWrapper`
- **Problema:** O wrapper exigia conversão imediata para Pandas/Polars para operações simples de inspeção.
- **Resolução Implementada:**
  - **`shape`**: Tupla `(linhas, colunas)` padrão NumPy/Pandas.
  - **`head(n=5)` e `tail(n=5)`**: Fatiamento instantâneo O(1) Zero-Copy via `batch.slice(...)`.
  - **`__getitem__`**: Acesso direto a colunas (`batch["IDADE"]` ou `batch[0]`) com delegação PyArrow/Polars.
  - **`to_dict()` / `to_pydict()`**: Conversão para dicionário padrão Python `{coluna: [valores]}`.

### 3.4 Multi-Jurisdições e Multi-Anos
- **Problema:** Necessidade de loops manuais no Python para consultar múltiplos estados (ex.: `["SP", "RJ", "MG"]`).
- **Resolução Implementada:** O `HospitalMorbidityAccessor.fetch()` agora aceita `jurisdictions: list[str]`, baixando e concatenando lotes colunares de forma transparente.

### 3.5 Tratamento de Erros Tipado
- **Problema:** Falhas de rede ou recursos não encontrados chegavam como `ValueError` genérico.
- **Resolução Implementada:** Exceções específicas tipadas herdando de `BRHealthError`:
  - `SourceNotFoundError`: Quando o identificador da fonte ou recurso não existe.
  - `TransportError`: Falhas de timeout ou conexão HTTP/FTP.
  - `ValidationError`: Incompatibilidades de esquema ou regras biológicas.

---

## 4. Comparação: Fluxo Antigo vs. Fluxo Otimizado

### Fluxo Antigo (v0.1.0 inicial):
```python
import brhealth

engine = brhealth.Engine()
# IDs manuais, sem autocomplete, trava kernel do Colab em downloads longos
try:
    batch = engine.fetch(source_id="datasus.sih", jurisdiction="SP", year=2023)
except Exception as e:
    print(e)  # String parsing frágil
df = batch.to_pyarrow().to_pandas()
```

### Fluxo Otimizado (v0.2.0):
```python
import brhealth

engine = brhealth.Engine()

# Autocomplete nativo no VS Code/Jupyter, não trava o kernel (GIL liberado),
# download multi-UF unificado e amostragem instantânea sem cópia:
try:
    batch = engine.hospital_morbidity.fetch(
        jurisdictions=["SP", "RJ", "MG"],
        years=[2022, 2023],
    )
    # Inspeção rápida em memória sem cópia
    print(batch.shape)  # (N, M)
    preview = batch.head(5)

    # Conversão flexível para qualquer destino analítico
    df_pandas = batch.to_pandas()
    df_polars = batch.to_polars()
    dict_dados = batch.to_dict()

    # Indexação direta de coluna
    idades = batch["IDADE"]
except brhealth.TransportError as e:
    print(f"Instabilidade no transporte: {e}")
except brhealth.SourceNotFoundError as e:
    print(f"Fonte não cadastrada: {e}")
```

---

## 5. Roteiro de Evolução da API Python

| # | Item | Benefício para Ciência de Dados | Status |
|---|---|---|---|
| 1 | Liberação do GIL (`py.allow_threads`) | Execução paralela em threads Python e kernel responsivo ao `Ctrl+C` | ✅ Concluído |
| 2 | Type Stubs (`.pyi` + `py.typed`) | Autocomplete e validação estática no Jupyter, Colab e VS Code | ✅ Concluído |
| 3 | Exceções Tipadas (`BRHealthError`, `SourceNotFoundError`, etc.) | Tratamento idiomático de falhas em pipelines analíticos | ✅ Concluído |
| 4 | Métodos de exploração rápida (`shape`, `head`, `tail`, `to_dict`, `__getitem__`) | Inspeção de microdados sem custo de memória adicional | ✅ Concluído |
| 5 | Suporte a `jurisdictions=[...]` no SIH-SUS | Elimina loops manuais para agregação regional | ✅ Concluído |
| 6 | Publicação do Release `0.2.0` no PyPI | Entrega da superfície pública completa via `pip install` | ⏳ Pendente de Release |
| 7 | CI com validação de `dir(brhealth)` no wheel | Impede que funções registradas no Rust fiquem de fora do pacote publicado | ⏳ Próximo |
| 8 | Interoperabilidade Zero-Copy com DuckDB (`to_duckdb`) | Execução de consultas SQL de alta performance em microdados | ⏳ Backlog |
| 9 | Particionamento e Streaming (`fetch_batches`) | Consumo de décadas de dados do SUS sem esgotar a RAM | ⏳ Backlog |
| 10 | Descoberta interativa de filtros (`available_filters`) | Documentação das opções de filtro em tempo de execução no notebook | ⏳ Backlog |
