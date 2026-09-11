<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# Exemplos de Uso do BRHealth (Python & Rust)

Esta pasta contém exemplos práticos executáveis demonstrando os recursos analíticos, bioestatísticos, geoespaciais e de decodificação nativa do **BRHealth**.

---

## 🐍 Exemplos em Python

Para executar os exemplos em Python, ative o ambiente virtual do projeto e utilize o interpretador com `brhealth` instalado:

```bash
# Ativação do ambiente virtual
source .venv/bin/activate

# Execução individual de qualquer exemplo
python examples/datasus_blast_zero_copy.py
```

### Índice de Scripts Python

| Arquivo | Domínio | Destaques Técnicos |
| :--- | :--- | :--- |
| [`datasus_blast_zero_copy.py`](datasus_blast_zero_copy.py) | **Decodificação DATASUS & Zero-Copy** | Descompressão nativa Blast DCL (`.dbc`), leitura colunar de `.dbf`, e travessia sem cópia para **Polars**, **Pandas** e **PyArrow**. |
| [`csap_health_economics_roi.py`](csap_health_economics_roi.py) | **Economia da Saúde & CSAP** | Classificação nos 19 grupos da **Portaria MS/SAS nº 221/2008**, custos evitáveis, dias de internação evitáveis e modelagem econométrica de **ROI da Atenção Primária**. |
| [`epidemiology_bioestatistica_apvp.py`](epidemiology_bioestatistica_apvp.py) | **Bioestatística & Ontologias** | Anos Potenciais de Vida Perdidos (**APVP/YLL**), padronização direta da taxa de mortalidade (**População Padrão OMS**), consistência biológica, mapeamento CID-9/10/11/SNOMED e procedimentos SIGTAP. |
| [`spatial_ibge_h3_s2.py`](spatial_ibge_h3_s2.py) | **Geoprocessamento & Harmonização** | Validação municipal **IBGE** (Luhn Módulo 10), reconciliação histórica (1970–2026), grade hexagonal **Uber H3**, **Google S2** e detecção de clusters de arboviroses. |
| [`engine_catalog_provenance.py`](engine_catalog_provenance.py) | **Catálogo & Governança FAIR** | Arquitetura do `brhealth.Engine()`, navegação pelas 26 fontes federadas, 8 acessores semânticos, gestão do cache Hive-Parquet e linhagem criptográfica **W3C PROV-O**. |

---

## 🦀 Exemplos em Rust

Os exemplos em Rust operam diretamente sobre o núcleo de alta performance (`brhealth-core`):

```bash
# Executar o pipeline completo do SIM / DATASUS em Rust
cargo run --example sim_datasus_full_pipeline

# Executar a avaliação de CSAP e ROI em Rust
cargo run --example csap_health_economics_roi
```

| Arquivo | Domínio | Descrição |
| :--- | :--- | :--- |
| [`sim_datasus_full_pipeline.rs`](sim_datasus_full_pipeline.rs) | **Pipeline Analítico SIM** | Ingestão com descompressão Blast DCL, harmonização IBGE, indexação H3, persistência Hive-Parquet com Time-Travel e emissão de manifesto criptográfico FAIR. |
| [`csap_health_economics_roi.rs`](csap_health_economics_roi.rs) | **Morbidade Hospitalar SIH** | Classificação colunar de CSAP conforme Portaria 221/2008, indicadores de diárias e custos evitáveis, e avaliação de ROI. |

---

## 📐 Fundamentação Teórica e Formulações Matemáticas

### 1. Retorno sobre Investimento (ROI) em Saúde Coletiva
$$\text{ROI}_{\text{APS}} = \frac{(\alpha \cdot \text{Custo Evitável}) - \text{Investimento}_{\text{APS}}}{\text{Investimento}_{\text{APS}}}$$
Onde $\alpha \in [0.0, 1.0]$ representa a Fração Atribuível Evitável das internações sensíveis à Atenção Primária.

### 2. Anos Potenciais de Vida Perdidos (APVP / YLL)
$$\text{APVP} = \sum_{i=1}^{n} d_i \cdot (L - a_i), \quad d_i = 1 \text{ se } a_i < L$$
Onde $L$ é a idade limite (70 anos pelo Ministério da Saúde ou 75 anos pela OMS).

### 3. Taxa Padronizada Direta de Mortalidade (OMS)
$$\text{TME}_{\text{OMS}} = \sum_{k=1}^{18} w_k \cdot \left( \frac{D_k}{P_k} \right) \times 100.000$$
Onde $w_k$ são os pesos das 18 faixas etárias quinquenais da População Padrão Mundial da Organização Mundial da Saúde.

### 4. Dígito Verificador Territorial IBGE (Luhn Módulo 10)
$$s = \sum_{i=1}^{6} \left( \lfloor (w_i \cdot c_i) / 10 \rfloor + ((w_i \cdot c_i) \bmod 10) \right), \quad w = (1, 2, 1, 2, 1, 2)$$
$$\text{DV} = (10 - (s \bmod 10)) \bmod 10$$
