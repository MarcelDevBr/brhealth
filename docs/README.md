<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

# 🏥 BRHealth

<p align="center">
  <strong>Motor Analítico Colunar de Alta Performance para Saúde Coletiva e Determinantes Sociais</strong>
</p>

<p align="center">
  <a href="PYTHON_QUICKSTART.md"><img src="https://img.shields.io/badge/python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13%20%7C%203.14-blue.svg" alt="Python Version"></a>
  <a href="USAGE_GUIDE.md"><img src="https://img.shields.io/badge/rust-2024%20edition-orange.svg" alt="Rust Edition"></a>
  <a href="architecture/sdd_brhealth.md"><img src="https://img.shields.io/badge/apache%20arrow-contiguous%2064--byte-red.svg" alt="Apache Arrow"></a>
  <a href="https://github.com/MarcelDevBr/brhealth/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-AGPLv3-green.svg" alt="License"></a>
  <a href="architecture/sdd_brhealth.md"><img src="https://img.shields.io/badge/FAIR-W3C%20PROV--O-purple.svg" alt="FAIR Principles"></a>
</p>

<p align="center">
  <a href="PYTHON_QUICKSTART.md" class="md-button md-button--primary">⚡ Começar com Python</a>
  <a href="USAGE_GUIDE.md" class="md-button">📖 Manual Completo</a>
  <a href="INSTALLATION.md" class="md-button">📦 Instalação</a>
  <a href="https://github.com/MarcelDevBr/brhealth" class="md-button">⭐ Repositório no GitHub</a>
</p>

---

## 🧭 Pilares do Motor Analítico

<div class="grid cards" markdown>

-   __⚡ Descompressão Blast DCL Nativa__

    ---

    Decodificação de arquivos comprimidos `.dbc` e `.dbf` do DATASUS 100% nativa em Rust. Elimina dependências legadas externas em C e conecta com **Polars**, **Pandas** e **PyArrow** via Zero-Copy.

    [:octicons-arrow-right-24: Ver Guia de Ingestão](PYTHON_QUICKSTART.md#21-ingestao-automatizada-e-cache-first-sem-download-manual)

-   __🏥 CSAP & Economia da Saúde__

    ---

    Classificação colunar nos 19 grupos da **Portaria MS/SAS nº 221/2008**, apuração de taxas por 10.000 hab., custos hospitalares evitáveis e modelagem econométrica de **ROI da Atenção Primária**.

    [:octicons-arrow-right-24: Ver Guia de CSAP](PYTHON_QUICKSTART.md#23-identificar-csap-e-calcular-roi-hospitalar)

-   __📊 Bioestatística & Mortalidade Prematura__

    ---

    Cálculo de Anos Potenciais de Vida Perdidos (**APVP / YLL**), Padronização Direta com a População Mundial da OMS (18 faixas quinquenais) e validação de consistência biológica de microdados.

    [:octicons-arrow-right-24: Ver Bioestatística](PYTHON_QUICKSTART.md#24-calcular-apvp-e-taxa-padronizada-de-mortalidade)

-   __🗺️ Geoprocessamento & Grades Discretas__

    ---

    Harmonização de municípios **IBGE** (algoritmo de Luhn Módulo 10, reconciliação 1970–2026), indexação hexagonal **Uber H3** e esférica **Google S2** para detecção de clusters e hotspots.

    [:octicons-arrow-right-24: Ver Geoprocessamento](PYTHON_QUICKSTART.md#25-indexação-espacial-com-uber-h3-e-google-s2)

</div>

---

## 🎯 Por Onde Começar? (Selecione seu Perfil)

```
                       ┌─────────────────────────────────────┐
                       │      QUAL É O SEU OBJETIVO?         │
                       └──────────────────┬──────────────────┘
                                          │
         ┌────────────────────────────────┼────────────────────────────────┐
         ▼                                ▼                                ▼
  [ Cientista de Dados ]       [ Epidemiologista / Gestor ]      [ Engenheiro de Software ]
  • Quer ler DBC sem dor       • Quer calcular CSAP e ROI        • Quer entender a arquitetura
  • Usa Polars / Pandas        • Quer APVP e taxas OMS           • Hexagonal DOD em Rust
  • Quer modelos em ML         • Precisa de dados do IBGE        • Zero-Copy FFI C-API
         │                                │                                │
         ▼                                ▼                                ▼
  [⚡ Python Quickstart]         [📊 Guia de CSAP & Métricas]     [📐 SDD & Arquitetura]
```

- 🔬 **Cientistas de Dados & Pesquisadores**:  
  Inicie pelo [**⚡ Guia Rápido de Python (Quickstart)**](PYTHON_QUICKSTART.md) e pelo [**📘 Guia Definitivo de Python**](PYTHON_GUIDE.md).
- 🏥 **Epidemiologistas, Bioestatísticos & Gestores Públicos**:  
  Consulte o [**Manual Prático de Uso**](USAGE_GUIDE.md) e o estudo de [**Custos Hospitalares de CSAP**](architecture/exemplo_analise_custos_csap.md).
- ⚙️ **Engenheiros de Dados & Desenvolvedores de Sistemas**:  
  Consulte o [**Guia de Instalação Modular**](INSTALLATION.md), o [**Software Design Document (SDD)**](architecture/sdd_brhealth.md) e o guia de [**Extensão de Novas Fontes (SPI)**](EXTENDING_BRHEALTH.md).

---

## 📚 Menu da Documentação por Domínio

???+ note "🐍 1. Ecossistema Python e Ciência de Dados"
    - [⚡ Guia Rápido de Python (Quickstart & Cookbook)](PYTHON_QUICKSTART.md): Receitas práticas de 3 linhas, "Como faço para..." e Cheat Sheet de bolso.
    - [📘 Guia Definitivo do BRHealth em Python](PYTHON_GUIDE.md): Manual exaustivo com arquitetura de memória, Google Colab e benchmarks.
    - [📋 API de Dados para Pesquisadores](BRHealth_API_Python_Ciencia_de_Dados.md): Especificação dos esquemas tabulares e manipulação de `RecordBatchWrapper`.
    - [💛 Notebook Demonstrativo no Google Colab](https://colab.research.google.com/github/MarcelDevBr/brhealth/blob/main/examples/setembro_amarelo_perfil_epidemiologico.ipynb): Estudo epidemiológico de lesões autoprovocadas no Brasil.
    - [📂 Scripts de Exemplo na pasta examples/](https://github.com/MarcelDevBr/brhealth/tree/main/examples): Códigos executáveis prontos para uso.

???+ note "🏥 2. Bioestatística, Epidemiologia e Economia da Saúde"
    - [📑 Manual Geral de Utilização e Métricas Científicas](USAGE_GUIDE.md): Formulações em LaTeX da Portaria MS/SAS nº 221/2008, ROI em saúde coletiva, APVP e padronização direta da OMS.
    - [💰 Estudo Analítico de Custos Hospitalares de CSAP](architecture/exemplo_analise_custos_csap.md): Aplicação empírica em microdados reais da Morbidade Hospitalar do SUS (SIH/AIH).
    - [🧬 Ontologias Médicas e Terminologias Clínicas](PYTHON_GUIDE.md#7-ontologias-médicas-farmácia-e-validação-biológica): Mapeamentos CID-9 $\to$ CID-10 $\to$ CID-11 $\to$ SNOMED-CT, SIGTAP e farmacologia ATC/RxNorm.

???+ note "🗺️ 3. Geoprocessamento e Harmonização Territorial"
    - [🏛️ Harmonização Territorial do IBGE](PYTHON_GUIDE.md#3-harmonização-e-validação-territorial-ibge): Validação por Luhn Módulo 10, conversão de 6 para 7 dígitos e reconciliação histórica (1970–2026).
    - [⬢ Grades Espaciais Discretas (Uber H3 & Google S2)](PYTHON_GUIDE.md#6-geoprocessamento-e-indexação-espacial-uber-h3--google-s2): Indexação de eventos de saúde para agregação territorial e detecção de hotspots.

???+ note "💾 4. Decodificadores Nativos e Formatos Legados do DATASUS"
    - [⚙️ Descompressor Blast PKWARE DCL (.dbc)](USAGE_GUIDE.md#3-decodificador-nativo-blast-dcl-dbc): Descompressor 100% nativo em Rust sem invocar `dbc2dbf`.
    - [📊 Leitor Colunar dBase (.dbf)](USAGE_GUIDE.md#4-leitor-colunar-dbf-arrow): Ingestão direta em `RecordBatch` Apache Arrow com tratamento de encodings.
    - [🗄️ Camada de Cache Hive-Parquet](architecture/sdd_brhealth.md): Particionamento analítico com suporte a Time-Travel e snapshots.

???+ note "🌐 5. Catálogo Federado de Fontes (26 Fontes)"
    - Conexão nativa com bases nacionais e globais:
      - **DATASUS**: SIM, SIH, SINASC, SINAN, SIA, CNES, SIPNI, SISVAN, SISCAN, BPS.
      - **IBGE**: Censo Demográfico, PNAD Contínua, POF, PENSE, MUNIC.
      - **Clima & Meio Ambiente**: INMET, BDQueimadas, PRODES, Siságua.
      - **Social**: MDS CadÚnico.
      - **Internacional**: WHO GHO, Copernicus ERA5, OpenAQ, PAHO PLISA, IHME GBD, WorldPop.
    - [🔌 Guia de Extensão de Novas Fontes (SPI)](EXTENDING_BRHEALTH.md): Como implementar novas fontes com a trait assíncrona `HealthDataSourceSPI`.

???+ note "🏛️ 6. Engenharia de Software, Arquitetura e Governança FAIR"
    - [🏗️ Software Design Document (SDD)](architecture/sdd_brhealth.md): Especificação completa da Arquitetura Hexagonal DOD, C Data Interface FFI e gestão de buffers.
    - [🔒 Governança de Dados e Conformidade com a LGPD](LGPD_GOVERNANCE.md): Protocolos de anonimização e anonimato territorial $k$-anônimo para dados de saúde.
    - [🌐 Linhagem Criptográfica FAIR (W3C PROV-O)](architecture/sdd_brhealth.md#manifesto-fair-e-grafo-w3c-prov-o): Geração de manifestos JSON-LD com hashes SHA-256 de dados brutos.
    - [🔄 CI/CD e Automação](CICD_AND_WORKFLOWS.md): Pipelines de testes automatizados e empacotamento.
    - [💡 Manifesto Conceitual](architecture/ideia.md): Proposta de valor e fundamentos da plataforma.

---

<div align="center">
  <sub>BRHealth • Aceleração Científica para a Saúde Pública Brasileira • Licenciado sob AGPLv3</sub>
</div>
