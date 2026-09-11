<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Guia Definitivo do BRHealth em Python (Google Colab, Jupyter e Scripts)

O **BRHealth** (`import brhealth`) é o motor analítico colunar de alta performance para dados de saúde coletiva, bioestatística, determinantes sociais e vigilância epidemiológica no Brasil.

Construído 100% em Rust sob a arquitetura **Hexagonal Orientada a Dados (Hexagonal DOD)**, o BRHealth conecta-se sem cópia de memória (**Zero-Copy**) com **Pandas**, **Polars**, **PyArrow** e **PyTorch**, eliminando dependências legadas em C, comandos de terminal lentos e gargalos de I/O.

---

## 1. Instalação e Configuração

### 1.1 No Google Colab ou Jupyter Notebook

Execute na primeira célula do seu notebook:

```python
# Instalação direta do repositório
!pip install git+https://github.com/MarcelDevBr/brhealth.git#subdirectory=crates/brhealth-python

# Ou, caso o pacote esteja publicado no PyPI:
# !pip install brhealth

# Bibliotecas analíticas recomendadas (opcionais para Zero-Copy)
!pip install polars pyarrow pandas
```

### 1.2 Verificação da Instalação

```python
import brhealth

print(f"BRHealth versão: {brhealth.__version__}")
```

---

## 2. Decodificação Nativa de Arquivos DATASUS (.dbc e .dbf)

Uma das maiores dificuldades históricas da comunidade de dados de saúde no Brasil é ler arquivos `.dbc` comprimidos pelo algoritmo Blast PKWARE DCL do DATASUS. O BRHealth resolve isso de forma **100% nativa em Rust, sem binários externos (`dbc2dbf`) ou wrappers em C**.

### 2.1 Leitura Direta de `.dbc` para Pandas, Polars ou PyArrow

```python
import brhealth

# Lê o arquivo comprimido do DATASUS diretamente da pasta local ou do Drive montado no Colab
batch = brhealth.read_dbc("RDSP2401.dbc")

print(f"Total de registros: {len(batch)} linhas")
print(f"Colunas encontradas: {batch.columns[:10]}")

# 1. Exportação Zero-Copy para Pandas (Ideal para o Google Colab)
df_pandas = batch.to_pandas()
display(df_pandas.head())

# 2. Exportação Zero-Copy para Polars (Máxima performance e multithreading)
df_polars = batch.to_polars()
print(df_polars.glimpse())

# 3. Exportação Zero-Copy para PyArrow
pa_table = batch.to_pyarrow()
```

### 2.2 Leitura Direta de Tabelas `.dbf`

```python
# Lê arquivo dBase III/IV (.dbf) diretamente em memória contígua Apache Arrow
batch_dbf = brhealth.read_dbf("municipios.dbf")
df = batch_dbf.to_pandas()
```

### 2.3 Descompressão de `.dbc` para Arquivo ou Bytes `.dbf`

Caso necessite apenas do arquivo `.dbf` descompactado para outro software ou visualizador:

```python
# Descomprime o DBC salvando como DBF canônico no disco
dbf_bytes = brhealth.decompress_dbc("RDSP2401.dbc", output_path="RDSP2401.dbf")
print(f"Tamanho do DBF descomprimido: {len(dbf_bytes) / (1024 * 1024):.2f} MB")
```

---

## 3. Harmonização e Validação Territorial (IBGE)

### 3.1 Validação e Cálculo de Dígito Verificador (Luhn Módulo 10)

Muitas bases do SUS registram municípios com 6 dígitos (código base), omitindo o 7º dígito verificador, o que quebra junções tabulares com censos e projeções populacionais do IBGE.

```python
import brhealth

# 1. Validação de código de 6 ou 7 dígitos (aceita str ou int)
assert brhealth.validate_ibge_code("3550308") == True   # São Paulo/SP (Válido)
assert brhealth.validate_ibge_code(3550308) == True     # Aceita int
assert brhealth.validate_ibge_code("3550309") == False  # DV incorreto (deveria ser 8)
assert brhealth.validate_ibge_code("355030") == True    # 6 dígitos válidos

# 2. Cálculo do Dígito Verificador canônico oficial
dv_sp = brhealth.calculate_ibge_dv("355030") # Retorna 8
dv_bh = brhealth.calculate_ibge_dv("310620") # Retorna 0 (Belo Horizonte/MG)
print(f"DV São Paulo: {dv_sp}, DV Belo Horizonte: {dv_bh}")

# 3. Harmonização direta para 7 dígitos canônicos
codigo_canônico = brhealth.harmonize_ibge_code("355030") # "3550308"
```

### 3.2 Reconciliação Histórica de Municípios (1970–2026)

Trata transições territoriais históricas, como a cisão de Goiás para a criação do estado do Tocantins (CF/88) ou a extinção do Território Federal de Fernando de Noronha.

```python
# Antigo Território Federal de Fernando de Noronha (200001) -> Incorporado a Pernambuco (2605459)
canonico_fn = brhealth.reconcile_historical_ibge_code("200001", reference_year=1980)
print(f"Fernando de Noronha contemporâneo: {canonico_fn}") # '2605459'

# Municípios históricos de Goiás transferidos para o Tocantins em 1988
araguaina = brhealth.reconcile_historical_ibge_code("520210", reference_year=1985)
print(f"Araguaína contemporânea (TO): {araguaina}") # '1702107'
```

---

## 4. Condições Sensíveis à Atenção Primária (CSAP) e ROI em Saúde

O BRHealth implementa estritamente a **Lista Brasileira de Internações por Condições Sensíveis à Atenção Primária** estabelecida pela **Portaria MS/SAS nº 221, de 17 de abril de 2008**.

### 4.1 Classificação de Diagnósticos CID-10

```python
import brhealth

# Verificação se o diagnóstico pertence à lista oficial de CSAP
assert brhealth.is_csap("J45.0") == True   # Asma
assert brhealth.is_csap("I10") == True     # Hipertensão Essencial
assert brhealth.is_csap("S06.0") == False  # Concussão cerebral (Trauma / Não-CSAP)

# Identificação do Grupo CSAP (1 a 19)
grupo_asma = brhealth.classify_cid10("J45.0") # Retorna 7
nome_grupo = brhealth.csap_group_name(grupo_asma)
print(f"Grupo {grupo_asma}: {nome_grupo}") # 'Asma'

grupo_has = brhealth.classify_cid10("I10") # Retorna 9
print(f"Grupo {grupo_has}: {brhealth.csap_group_name(grupo_has)}") # 'Hipertensão arterial sistêmica'
```

### 4.2 Avaliação Econômica e Retorno sobre o Investimento (ROI)

$$ROI = \frac{\text{Custo Evitado} \times \text{Fração Atribuível} - \text{Investimento}}{\text{Investimento}}$$

```python
# Cálculo do ROI de ampliação de Equipes de Saúde da Família (eSF)
# Exemplo: R$ 500.000 de custos com internações evitáveis, R$ 100.000 de investimento na APS,
# e fração atribuível de 50% de redução nas hospitalizações.
roi = brhealth.compute_roi(
    avoidable_cost=500_000.0,
    investment=100_000.0,
    attributable_fraction=0.50
)

print(f"Retorno sobre Investimento (ROI): {roi * 100:.1f}%") # 150.0%
```

---

## 5. Bioestatística, Mortalidade e APVP

### 5.1 Anos Potenciais de Vida Perdidos (APVP / YLL)

Calcula a carga de mortalidade prematura abaixo de uma idade limite de referência ($L = 70$ anos conforme Ministério da Saúde, ou $L = 75$ pela OMS):

$$\text{APVP} = \sum_{i=1}^{n} d_i \cdot (L - a_i), \quad d_i = 1 \text{ se } a_i < L$$

```python
import brhealth

# Idades de óbito observadas em uma coorte de estudo
idades_obito = [12, 28, 45, 68, 72, 85]

# Cálculo de APVP com idade limite de 70 anos
apvp_total = brhealth.compute_apvp(idades_obito, cutoff_age=70)
print(f"Total de Anos de Vida Perdidos: {apvp_total} anos")
# (70-12) + (70-28) + (70-45) + (70-68) = 58 + 42 + 25 + 2 = 127 anos

# Taxa de APVP por 100.000 habitantes
taxa_apvp = brhealth.compute_apvp_rate(total_apvp=apvp_total, population=50_000)
print(f"Taxa de APVP: {taxa_apvp:.2f} por 100.000 hab.")
```

### 5.2 Cálculo de APVP em Lote sobre um `RecordBatch`

Caso tenha carregado uma base com milhões de óbitos do SIM:

```python
# Calcula estatísticas completas de APVP vetorizadamente em C++/Rust
metricas = brhealth.compute_batch_apvp(
    wrapper=batch_sim,
    age_column="IDADE",
    cutoff_age=70,
    reference_population=1_200_000
)

print("Métricas de Mortalidade Prematura:")
print(f"- Total APVP: {metricas['total_apvp']:,} anos")
print(f"- Óbitos Prematuros: {metricas['premature_deaths']:,}")
print(f"- Média de Anos Perdidos por Óbito: {metricas['mean_years_lost_per_death']:.1f} anos")
print(f"- Taxa Padronizada por 100k hab: {metricas['apvp_rate_per_100k']:.2f}")
```

### 5.3 Padronização Direta de Mortalidade (População Padrão OMS)

Elimina o viés de estruturas etárias dissimilares entre municípios aplicando os 18 pesos da População Padrão Mundial da OMS (0 a 85+ anos):

```python
# Vetores com 18 faixas etárias quinquenais (0-4, 5-9, ..., 80-84, 85+)
obitos_por_faixa = [5, 2, 3, 4, 8, 12, 15, 20, 30, 45, 60, 80, 110, 140, 180, 210, 190, 150]
populacao_local = [5000] * 18

taxa_padronizada = brhealth.compute_age_standardized_mortality_rate(
    observed_deaths=obitos_por_faixa,
    local_pop=populacao_local
)

print(f"Taxa Padronizada Direta da OMS: {taxa_padronizada:.2f} por 100.000 hab.")
```

---

## 6. Geoprocessamento e Indexação Espacial (Uber H3 & Google S2)

O BRHealth indexa eventos epidemiológicos e ambientais em células espaciais discretas de 64 bits (`uint64`), acelerando agregações territoriais e detecção de aglomerados (*clusters*).

### 6.1 Malha Hexagonal Uber H3

```python
import brhealth

# Coordenadas do Marco Zero de São Paulo (Praça da Sé)
lat, lon = -23.550520, -46.633308

# 1. Converte (lat, lon) em célula H3 (Resolução 7 ~ 1,2 km de raio médio)
h3_index = brhealth.latlng_to_h3(lat, lon, resolution=7)
print(f"Índice H3: {hex(h3_index)}")

# 2. Converte o índice de volta para o centróide
centro_lat, centro_lon = brhealth.h3_to_latlng(h3_index)
print(f"Centróide: lat={centro_lat:.6f}, lon={centro_lon:.6f}")

# 3. Raio de vizinhança espacial (anel de raio k=1 retorna 7 hexágonos)
vizinhos = brhealth.h3_grid_disk(h3_index, k=1)
print(f"Células vizinhas: {len(vizinhos)}")

# 4. Distância topológica na grade hexagonal
dist = brhealth.h3_grid_distance(vizinhos[0], vizinhos[1])
print(f"Distância entre células: {dist} passos")
```

### 6.2 Malha Esférica Google S2

```python
# Converte coordenadas para índice S2 CellId de 64 bits (Nível 10 padrão municipal)
s2_cell = brhealth.coord_to_s2_cell(lat, lon, level=10)
print(f"S2 Cell ID: {s2_cell}")

# Reversão do Cell ID para latitude e longitude
c_lat, c_lon = brhealth.s2_cell_to_coord(s2_cell)
```

---

## 7. Ontologias Médicas, Farmácia e Validação Biológica

### 7.1 Mapeamento Cruzado entre Nomenclaturas Clínicas

```python
import brhealth

# CID-9 para CID-10 (estudos históricos e séries temporais longas)
print(brhealth.map_icd9_to_icd10("493"))  # 'J45' (Asma)
print(brhealth.map_icd9_to_icd10("410"))  # 'I21' (Infarto Agudo do Miocárdio)

# CID-10 para CID-11 (transição sanitária internacional)
print(brhealth.map_icd10_to_icd11("I10")) # 'BA00' (Hipertensão essencial)
print(brhealth.map_icd10_to_icd11("E11")) # '5A11' (Diabetes tipo 2)

# CID-10 para SNOMED-CT (prontuário eletrônico e semântica de interoperabilidade)
print(brhealth.map_icd10_to_snomed("I10")) # '38341003'

# Informações completas do Capítulo da CID-10
capitulo = brhealth.icd10_chapter("I10")
print(f"Capítulo: {capitulo['roman']} ({capitulo['number']}) - {capitulo['title_pt']}")
# Capítulo: IX (9) - Doenças do aparelho circulatório
```

### 7.2 Validação de Consistência Biológica

Detecta incongruências epidemiológicas estritas em microdados (incompatibilidades fisiológicas de sexo e idade):

```python
# Gravidez/Parto em sexo masculino
try:
    brhealth.validate_biological_consistency(icd10="O00", sex="M", age_years=30)
except ValueError as e:
    print(f"Inconsistência detectada com sucesso: {e}")

# Patologias senis/neurodegenerativas em recém-nascidos
try:
    brhealth.validate_biological_consistency(icd10="G30", sex="F", age_years=2)
except ValueError as e:
    print(f"Inconsistência de idade detectada: {e}")
```

### 7.3 Farmacologia (ATC / RxNorm) e Procedimentos SUS (SIGTAP)

```python
# Consulta de princípio ativo por código ATC da OMS
farmaco = brhealth.lookup_atc("A10BA02")
print(f"Princípio ativo: {farmaco}") # 'Metformina'

# Equivalência internacional ATC -> RxNorm
rxnorm = brhealth.map_atc_to_rxnorm("A10BA02")
print(f"RxNorm Concept: {rxnorm}") # '6809'

# Procedimentos especiais SIGTAP do SUS
assert brhealth.is_amputation_procedure("0407040101") == True
assert brhealth.is_dialysis_procedure("0305010107") == True

# Decomposição estrutural do código SIGTAP de 10 dígitos (Grupo, Subgrupo, Forma, Seq, DV)
grupo, subgrupo, forma, seq, dv = brhealth.parse_sigtap_code("0407040101")
print(f"SIGTAP: Grupo={grupo}, Subgrupo={subgrupo}, Sequencial={seq}, DV={dv}")
```

---

## 8. Ingestão Integrada de Dados com o `Engine`

O `brhealth.Engine` gerencia o catálogo de fontes de dados nacionais e globais, executando pipelines analíticos completos sob demanda.

### 8.1 Inicialização e Acessores Especializados

```python
import brhealth

# Inicializa o motor analítico padrão
engine = brhealth.Engine()
print(f"Fontes prontas para consulta: {engine.source_count()}")

# Lista de fontes registradas
for src in engine.list_sources()[:5]:
    print(f" - {src}")
```

### 8.2 Acessores Semânticos por Domínio

```python
# 1. Morbidade Hospitalar do SUS (SIH / AIH)
batch_sih = engine.hospital_morbidity.fetch(
    jurisdiction="SP",
    year=2024,
    month=1,
    harmonize_ibge=True,
    enrich_csap=True
)
print(f"Internações SIH carregadas: {len(batch_sih)}")

# 2. Estatísticas Vitais (SIM e SINASC)
batch_sim = engine.vital_statistics.fetch(source="SIM", jurisdiction="RJ", year=2023)
batch_sinasc = engine.vital_statistics.fetch(source="SINASC", jurisdiction="MG", year=2023)

# 3. Vigilância Epidemiológica (SINAN)
batch_dengue = engine.notifications.fetch(disease="DENG", jurisdiction="BA", year=2024)

# 4. Dados Censitários e Demográficos (IBGE)
batch_censo = engine.demographics.fetch(source="censo", jurisdiction="SP", year=2022)

# 5. Atenção Ambulatorial e Estabelecimentos (SIA e CNES)
batch_cnes = engine.ambulatory.fetch(source="CNES", jurisdiction="DF", year=2024)

# 6. Clima e Ambiente (INMET, BDQueimadas, Prodes, Siságua)
batch_queimadas = engine.environmental.fetch(source="bdqueimadas", year=2024)

# 7. Clima Global Reanálise Copernicus ERA5
batch_era5 = engine.global_climate.fetch_reanalysis(year=2024, month=6)

# 8. Vulnerabilidade Social e Cadastro Único (MDS)
batch_cadunico = engine.social.fetch(jurisdiction="CE", year=2024)
```

### 8.3 Função Top-Level `brhealth.fetch()`

Caso não queira instanciar o `Engine()` manualmente, o BRHealth disponibiliza a função direta:

```python
batch = brhealth.fetch(
    source_id="datasus.sih",
    jurisdiction="PR",
    year=2024,
    month=2,
    harmonize_ibge=True,
    enrich_csap=True
)
df = batch.to_pandas()
```

---

## 9. Interoperabilidade Zero-Copy e Machine Learning

O `RecordBatchWrapper` expõe a **Arrow C Data Interface** (`__arrow_c_array__`) e o protocolo **DLPack** (`__dlpack__`), permitindo transferir milhões de linhas para tensores sem overhead de serialização:

```python
import brhealth
import torch

batch = brhealth.read_dbc("exemplo.dbc")

# Passagem direta para PyTorch via DLPack sem cópia intermediária de buffers
tensor = torch.from_dlpack(batch)
print(f"Tensor Shape: {tensor.shape}, Dispositivo: {tensor.device}")
```

---

## 10. Resumo de Todas as Funções e Classes Disponíveis

| Assinatura | Tipo | Descrição |
| :--- | :--- | :--- |
| `read_dbc(path)` | Função | Descomprime e decodifica arquivo `.dbc` do DATASUS diretamente para `RecordBatchWrapper`. |
| `read_dbf(path)` | Função | Decodifica tabela `.dbf` diretamente para `RecordBatchWrapper`. |
| `decompress_dbc(in, out=None)` | Função | Descomprime arquivo `.dbc` retornando bytes `.dbf` (e opcionalmente salva em disco). |
| `calculate_ibge_dv(code)` | Função | Calcula o Dígito Verificador oficial de 6 dígitos pelo Módulo 10 Luhn. |
| `harmonize_ibge_code(code)` | Função | Harmoniza código municipal de 6 ou 7 dígitos para 7 dígitos canônicos. |
| `validate_ibge_code(code)` | Função | Valida consistência matemática e cadastral de um código municipal do IBGE. |
| `reconcile_historical_ibge_code(code, year)` | Função | Reconcilia municípios de transições históricas (TO/GO, Fernando de Noronha, etc.). |
| `latlng_to_h3(lat, lng, res)` | Função | Converte coordenadas em índice hexagonal Uber H3 (uint64). |
| `h3_to_latlng(h3_index)` | Função | Converte índice H3 de volta para coordenadas do centróide (lat, lon). |
| `h3_grid_disk(h3_index, k)` | Função | Retorna lista de células vizinhas num anel/disco de raio $k$. |
| `h3_grid_distance(orig, dest)` | Função | Retorna distância em passos de hexágonos entre células H3. |
| `coord_to_s2_cell(lat, lon, level)` | Função | Converte coordenadas em Google S2 CellId de 64 bits. |
| `s2_cell_to_coord(cell_id)` | Função | Converte Google S2 CellId de volta para (lat, lon). |
| `compute_apvp(ages, cutoff)` | Função | Calcula o somatório de Anos Potenciais de Vida Perdidos (APVP / YLL). |
| `compute_apvp_rate(apvp, pop)` | Função | Calcula a taxa padronizada de APVP por 100.000 habitantes. |
| `compute_batch_apvp(batch, col, ...)` | Função | Calcula estatísticas completas de APVP vetorizadamente sobre um lote Arrow. |
| `compute_age_standardized_mortality_rate(...)`| Função | Calcula taxa padronizada direta de mortalidade segundo a população da OMS. |
| `classify_cid10(cid)` | Função | Classifica um código CID-10 em um dos 19 grupos de CSAP (Portaria 221/2008). |
| `is_csap(cid)` | Função | Verifica se um CID-10 pertence à Lista Brasileira de CSAP. |
| `csap_group_name(group_id)` | Função | Retorna o nome oficial em português de um grupo de CSAP (1 a 19). |
| `icd10_chapter(code)` | Função | Retorna metadados do capítulo da CID-10 (número, algarismo romano e título). |
| `validate_biological_consistency(cid, sex, age)` | Função | Valida consistência biológica estrita de patologia contra sexo e idade. |
| `map_icd9_to_icd10(code)` | Função | Mapeia código histórico CID-9 para CID-10 canônico. |
| `map_icd10_to_icd11(code)` | Função | Mapeia código CID-10 para CID-11. |
| `map_icd10_to_snomed(code)` | Função | Mapeia CID-10 para identificador SNOMED-CT internacional. |
| `is_amputation_procedure(code)` | Função | Verifica se procedimento SIGTAP é amputação de membro. |
| `is_dialysis_procedure(code)` | Função | Verifica se procedimento SIGTAP é diálise/terapia renal substitutiva. |
| `parse_sigtap_code(code)` | Função | Decompõe código de 10 dígitos do SIGTAP nos seus atributos hierárquicos. |
| `lookup_atc(code)` | Função | Retorna o princípio ativo de um código de medicamento da OMS (ATC). |
| `map_atc_to_rxnorm(code)` | Função | Mapeia código ATC para conceito RxNorm. |
| `compute_roi(cost, invest, frac)`| Função | Modela o Retorno sobre Investimento da Atenção Primária em custos evitados. |
| `fetch(source_id, ...)` | Função | Ingestão e harmonização direta via motor global singleton. |
| `Engine` | Classe | Motor analítico com acesso aos 8 accessors e registros de fontes. |
| `RecordBatchWrapper` | Classe | Contêiner colunar Zero-Copy Arrow (`to_pandas`, `to_polars`, `to_pyarrow`, etc.). |
