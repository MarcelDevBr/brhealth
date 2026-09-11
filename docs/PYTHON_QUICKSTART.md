<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# ⚡ Guia Rápido de Python (BRHealth Quickstart & Cookbook)

Seja bem-vindo ao **Guia Prático do BRHealth para Python**. Este documento foi elaborado para que você possa copiar, colar e rodar tarefas comuns de epidemiologia, economia da saúde e geoprocessamento em menos de 2 minutos.

---

## 📌 Sumário Rápido

1. [Instalação Expressa](#1-instalação-expressa)
2. [Receitas Rápidas ("Como faço para...")](#2-receitas-rápidas-como-faço-para)
   - [2.1 Ler arquivo `.dbc` ou `.dbf` do DATASUS direto para Polars ou Pandas](#21-ler-arquivo-dbc-ou-dbf-do-datasus)
   - [2.2 Validar ou harmonizar código de município do IBGE (6 vs 7 dígitos)](#22-validar-e-harmonizar-códigos-ibge)
   - [2.3 Checar se uma internação é evitável (CSAP) e calcular ROI da Atenção Primária](#23-identificar-csap-e-calcular-roi-hospitalar)
   - [2.4 Calcular Anos Potenciais de Vida Perdidos (APVP) e Taxa Padronizada da OMS](#24-calcular-apvp-e-taxa-padronizada-de-mortalidade)
   - [2.5 Converter latitude/longitude em hexágono Uber H3 ou Google S2](#25-indexação-espacial-com-uber-h3-e-google-s2)
   - [2.6 Consultar fontes e gerenciar cache pelo `Engine`](#26-consultar-o-catálogo-federado-com-o-engine)
3. [Cheat Sheet de Bolso (Tabela de Referência de Funções)](#3-cheat-sheet-de-bolso)
4. [Tratamento de Erros Comuns (Troubleshooting)](#4-tratamento-de-erros-comuns)

---

## 1. Instalação Expressa

No terminal ou na primeira linha do seu **Google Colab / Jupyter Notebook**:

```bash
# Instalação com suporte a Polars e PyArrow
pip install polars pyarrow pandas

# Instalação do BRHealth
pip install git+https://github.com/MarcelDevBr/brhealth.git#subdirectory=crates/brhealth-python
```

Para verificar se está tudo certo:

```python
import brhealth

print(f"BRHealth versão: {brhealth.__version__}")
# Diagnóstico de acelerações Zero-Copy disponíveis no seu sistema
print(brhealth.check_environment())
# {'pyarrow': True, 'polars': True, 'pandas': True, 'torch': False}
```

---

## 2. Receitas Rápidas ("Como faço para...")

### 2.1 Ler arquivo `.dbc` ou `.dbf` do DATASUS

> **Sem necessidade de binários externos ou `dbc2dbf`**. A descompressão é 100% nativa e rápida em Rust.

```python
import brhealth

# 1. Carregar arquivo comprimido do DATASUS (.dbc)
batch = brhealth.read_dbc("doac2022.dbc")

# 2. Converter sem cópia de memória (Zero-Copy) para seu DataFrame favorito:
df_polars = batch.to_polars()  # Para máxima velocidade e multithreading
df_pandas = batch.to_pandas()  # Para ecossistema Pandas / Seaborn
pa_table = batch.to_pyarrow()  # Para processamento colunar Arrow

print(f"Total de registros: {len(batch):,} linhas x {batch.num_columns} colunas")
print(df_polars.head(3))
```

Se quiser apenas descompactar um arquivo `.dbc` para `.dbf` no disco:

```python
brhealth.decompress_dbc("doac2022.dbc", output_path="doac2022.dbf")
```

---

### 2.2 Validar e harmonizar códigos IBGE

> O DATASUS frequentemente utiliza códigos municipais com **6 dígitos**, enquanto o IBGE e Censos utilizam **7 dígitos** (com o Dígito Verificador de Luhn Módulo 10).

```python
import brhealth

# Validar consistência matemática do código
assert brhealth.validate_ibge_code("3550308") == True  # São Paulo/SP (7 dígitos válido)
assert brhealth.validate_ibge_code("3550309") == False # DV adulterado (deveria ser 8)
assert brhealth.validate_ibge_code(3550308) == True    # Aceita int ou str

# Calcular o 7º dígito verificador (DV) oficial
dv = brhealth.calculate_ibge_dv("355030")  # Retorna 8

# Harmonizar de 6 para 7 dígitos canônicos
codigo_7d = brhealth.harmonize_ibge_code("355030")  # "3550308"

# Reconciliar municípios com mudanças históricas (ex: Goiás -> Tocantins em 1988)
araguaina_atual = brhealth.reconcile_historical_ibge_code("520210", reference_year=1985)
print(araguaina_atual)  # Retorna '1702107' (Araguaína contemporânea no TO)
```

---

### 2.3 Identificar CSAP e calcular ROI hospitalar

> Implementação estrita da **Portaria MS/SAS nº 221/2008** (Lista Brasileira de Internações por Condições Sensíveis à Atenção Primária).

```python
import brhealth

# 1. Checar se um CID-10 é evitável na Atenção Primária
print(brhealth.is_csap("J45.0"))  # True (Asma)
print(brhealth.is_csap("I10"))    # True (Hipertensão)
print(brhealth.is_csap("S06.0"))  # False (Trauma cerebral / Não evitável)

# 2. Descobrir qual dos 19 grupos oficiais de CSAP o diagnóstico pertence
grupo_id = brhealth.classify_cid10("J45.0")  # 7
print(f"Grupo {grupo_id}: {brhealth.csap_group_name(grupo_id)}")
# Grupo 7: Asma

# 3. Modelagem de Retorno sobre o Investimento (ROI)
# Exemplo: R$ 500.000 de custos com internações evitáveis de asma e hipertensão,
# investimento de R$ 100.000 em equipes da Saúde da Família (eSF),
# e redução estimada de 45% (fração atribuível evitável):
roi = brhealth.compute_roi(
    avoidable_cost=500_000.0,
    investment=100_000.0,
    attributable_fraction=0.45,
)
print(f"ROI da Atenção Primária: {roi * 100:.1f}%")
# Retorna 125.0% (cada R$ 1,00 investido na APS gera R$ 2,25 em benefícios)
```

---

### 2.4 Calcular APVP e taxa padronizada de mortalidade

```python
import brhealth

# Coorte com idades de óbito observadas
idades = [14, 22, 35, 48, 62, 68, 73, 81]

# 1. Anos Potenciais de Vida Perdidos (Limite do Ministério da Saúde = 70 anos)
apvp = brhealth.compute_apvp(idades, cutoff_age=70)
print(f"Total de Anos de Vida Perdidos: {apvp} anos")

# 2. Taxa de APVP por 100.000 habitantes
taxa_apvp = brhealth.compute_apvp_rate(total_apvp=apvp, population=50_000)
print(f"Taxa de APVP: {taxa_apvp:.2f} por 100k hab.")

# 3. Padronização Direta da Mortalidade (18 faixas quinquenais da OMS)
obitos = [10] * 18
populacao = [3000] * 18
taxa_oms = brhealth.compute_age_standardized_mortality_rate(obitos, populacao)
print(f"Taxa Padronizada Direta da OMS: {taxa_oms:.2f} por 100k hab.")

# 4. Validar consistência biológica estrita (impede incongruências em microdados)
try:
    brhealth.validate_biological_consistency(icd10="O00", sex="M", age_years=30)
except ValueError as e:
    print("Inconsistência biológica rejeitada com sucesso!")
```

---

### 2.5 Indexação espacial com Uber H3 e Google S2

> Permite transformar latitude e longitude em índices inteiros (`uint64`), acelerando agregações, detecção de surtos (hotspots) e cruzamentos territoriais.

```python
import brhealth

# Coordenadas (Praça da Sé - São Paulo/SP)
lat, lon = -23.550520, -46.633308

# 1. Obter índice hexagonal H3 (Resolução 8 ~ 0.7 km² de área média)
h3_index = brhealth.latlng_to_h3(lat, lon, resolution=8)
print(f"Célula H3: {hex(h3_index)}")

# 2. Obter anel de vizinhança espacial (1 centro + 6 hexágonos vizinhos)
vizinhos = brhealth.h3_grid_disk(h3_index, k=1)
print(f"Total de células no anel de raio 1: {len(vizinhos)}")

# 3. Obter índice esférico Google S2 (Nível 12 ~ 3 a 5 km²)
s2_cell = brhealth.coord_to_s2_cell(lat, lon, level=12)
print(f"S2 Cell ID: {s2_cell}")
```

---

### 2.6 Consultar o catálogo federado com o `Engine`

```python
from brhealth import Engine

engine = Engine()

print(f"Versão: {engine.version()}")
print(f"Fontes catalogadas: {engine.source_count()}")

# Listar primeiras fontes registradas
for fonte in engine.list_sources()[:5]:
    print(f" - {fonte}")

# Status e governança do cache Hive-Parquet local
print(engine.cache.status())
# Limpar snapshots de cache com mais de 60 dias
engine.cache.clear_older_than(days=60)
```

---

## 3. Cheat Sheet de Bolso

Tabela de consulta ultra rápida com as principais funções do módulo `brhealth`:

| Função | Argumentos | Retorno | Descrição |
| :--- | :--- | :--- | :--- |
| `read_dbc(path)` | `path: str` | `RecordBatchWrapper` | Descomprime arquivo `.dbc` e carrega em memória contígua Arrow. |
| `read_dbf(path)` | `path: str` | `RecordBatchWrapper` | Lê arquivo `.dbf` em lote colunar Arrow. |
| `decompress_dbc(in, out=None)` | `in: str, out: str?` | `bytes` | Descomprime `.dbc` retornando bytes e opcionalmente salva `.dbf`. |
| `calculate_ibge_dv(code)` | `str \| int` (6 dígitos) | `int` | Calcula o 7º dígito verificador oficial (Luhn Módulo 10). |
| `validate_ibge_code(code)` | `str \| int` (6 ou 7D) | `bool` | Valida consistência matemática e cadastral de município. |
| `harmonize_ibge_code(code)` | `str \| int` (6 dígitos) | `str` (7 dígitos) | Converte código de 6 dígitos em código canônico de 7 dígitos. |
| `reconcile_historical_ibge_code(code, year)` | `str \| int, int?` | `str` | Reconcilia transições territoriais históricas (1970–2026). |
| `is_csap(cid)` | `cid: str` | `bool` | Retorna `True` se o CID-10 for condição sensível à APS (Portaria 221/2008). |
| `classify_cid10(cid)` | `cid: str` | `int?` (1 a 19) | Identifica o número do grupo de CSAP oficial. |
| `csap_group_name(group_id)` | `group_id: int` | `str` | Retorna o nome oficial em português do grupo CSAP. |
| `compute_roi(cost, invest, alpha)` | `float, float, float` | `float` | Modela o Retorno sobre o Investimento da APS em custos evitados. |
| `compute_apvp(ages, cutoff=70)` | `list[int], int?` | `int` | Calcula o total de Anos Potenciais de Vida Perdidos (YLL). |
| `compute_apvp_rate(apvp, pop)` | `int, int` | `float` | Taxa de APVP por 100.000 habitantes. |
| `compute_age_standardized_mortality_rate(d, p)` | `list[int], list[int]` | `float` | Taxa direta padronizada com 18 faixas quinquenais da OMS. |
| `validate_biological_consistency(cid, sex, age)` | `str, str, int` | `bool` | Valida compatibilidade fisiológica de diagnóstico, sexo e idade. |
| `latlng_to_h3(lat, lon, res)` | `float, float, int` | `int` (uint64) | Converte coordenadas em célula hexagonal Uber H3. |
| `h3_to_latlng(h3_index)` | `int` (uint64) | `tuple[float, float]` | Centróide (lat, lon) de uma célula H3. |
| `h3_grid_disk(h3_index, k)` | `int, int` | `list[int]` | Retorna hexágonos no anel de vizinhança de raio $k$. |
| `coord_to_s2_cell(lat, lon, level)` | `float, float, int?` | `int` (uint64) | Converte coordenadas em Google S2 Cell ID. |
| `map_icd9_to_icd10(code)` | `str` | `str?` | Mapeia código histórico CID-9 para CID-10. |
| `map_icd10_to_icd11(code)` | `str` | `str?` | Mapeia código CID-10 para CID-11. |
| `map_icd10_to_snomed(code)` | `str` | `str?` | Mapeia CID-10 para SNOMED-CT Concept ID. |
| `lookup_atc(atc_code)` | `str` | `str?` | Retorna o princípio ativo farmacológico da OMS. |
| `map_atc_to_rxnorm(atc_code)` | `str` | `str?` | Mapeia código ATC para conceito internacional RxNorm. |

---

## 4. Tratamento de Erros Comuns

- **`ValueError: Inconsistência biológica: CID 'O00' atribuído a indivíduo do sexo masculino`**:
  O validador biológico do BRHealth barrou um registro incongruente. Use um bloco `try/except ValueError` para capturar inconsistências nos seus dados de limpeza.
- **`ValueError: Código IBGE inválido`**:
  O código possui caracteres não numéricos ou quantidade de dígitos diferente de 6 ou 7.
- **`AttributeError: 'RecordBatchWrapper' object has no attribute 'xyz'`**:
  Converta primeiro o lote para o seu DataFrame preferido usando `.to_polars()`, `.to_pandas()` ou `.to_pyarrow()`.
