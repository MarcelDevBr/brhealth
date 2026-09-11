<!--
Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
Licensed under the GNU Affero General Public License v3 (AGPLv3)
or a commercial license agreement directly with the author.
-->

<div align="center">

# 🏥 Portal de Documentação do BRHealth

**Motor Analítico Colunar de Alta Performance para Saúde Coletiva e Determinantes Sociais**

[![Python 3.10+](https://img.shields.io/badge/python-3.10%20%7C%203.11%20%7C%203.12%20%7C%203.13%20%7C%203.14-blue.svg)](PYTHON_QUICKSTART.md)
[![Rust 2024](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](USAGE_GUIDE.md)
[![Apache Arrow 54.3](https://img.shields.io/badge/apache%20arrow-contiguous%2064--byte-red.svg)](architecture/sdd_brhealth.md)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-green.svg)](../LICENSE)
[![FAIR Principles](https://img.shields.io/badge/FAIR-W3C%20PROV--O-purple.svg)](architecture/sdd_brhealth.md)

</div>

---

## 🧭 Barra de Navegação Rápida

<div align="center">

[🏠 Início](../README.md) • [⚡ Python Quickstart](PYTHON_QUICKSTART.md) • [🐍 Python Avançado](PYTHON_GUIDE.md) • [📖 Manual de Uso](USAGE_GUIDE.md) • [📦 Instalação](INSTALLATION.md) • [📐 Arquitetura](architecture/sdd_brhealth.md) • [📊 Exemplos](../examples/README.md)

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

- 🔬 **Cientistas de Dados & Pesquisadores (Python / Polars / PyTorch)**:  
  Inicie pelo [**⚡ Guia Rápido de Python (Quickstart)**](PYTHON_QUICKSTART.md) e consulte os [**Exemplos Práticos em Python**](../examples/README.md).
- 🏥 **Epidemiologistas, Bioestatísticos & Gestores Públicos**:  
  Consulte o [**Manual Prático de Uso**](USAGE_GUIDE.md), o estudo de [**Custos Hospitalares CSAP**](architecture/exemplo_analise_custos_csap.md) e o [**Guia de Governança e LGPD**](LGPD_GOVERNANCE.md).
- ⚙️ **Engenheiros de Dados & Desenvolvedores de Sistemas (Rust / C++ / Java)**:  
  Consulte o [**Guia de Instalação Modular**](INSTALLATION.md), o [**Software Design Document (SDD)**](architecture/sdd_brhealth.md) e o guia de [**Extensão de Novas Fontes de Dados (SPI)**](EXTENDING_BRHEALTH.md).

---

## 📚 Menu Interativo de Navegação da Documentação

Clique nos tópicos abaixo para expandir e navegar pelos manuais detalhados:

<details open>
<summary><h3>🐍 1. Ecossistema Python e Ciência de Dados</h3></summary>

Guias práticos, receitas passo a passo, integração Zero-Copy e cadernos interativos:

- ⚡ [**Guia Rápido de Python (Quickstart & Cookbook)**](PYTHON_QUICKSTART.md): O manual mais direto para o dia a dia. Inclui receitas de "Como faço para...", instalação rápida e **Cheat Sheet de Bolso** com todas as funções.
- 📘 [**Guia Definitivo do BRHealth em Python**](PYTHON_GUIDE.md): Documentação exaustiva com explicações teóricas, suporte a Google Colab e transferências Zero-Copy para **Pandas**, **Polars**, **PyArrow** e tensores **PyTorch** via DLPack.
- 📋 [**API de Dados para Pesquisadores**](BRHealth_API_Python_Ciencia_de_Dados.md): Especificação dos esquemas tabulares de saída e manipulação de `RecordBatchWrapper`.
- 📁 [**Exemplos Práticos em Python na Pasta `examples/`**](../examples/README.md):
  - [`datasus_blast_zero_copy.py`](../examples/datasus_blast_zero_copy.py) - Descompressão Blast DCL e integração com Polars/Pandas.
  - [`csap_health_economics_roi.py`](../examples/csap_health_economics_roi.py) - Avaliação de CSAP e modelagem de ROI.
  - [`epidemiology_bioestatistica_apvp.py`](../examples/epidemiology_bioestatistica_apvp.py) - Bioestatística, APVP, taxas padronizadas da OMS e ontologias.
  - [`spatial_ibge_h3_s2.py`](../examples/spatial_ibge_h3_s2.py) - Harmonização territorial IBGE, Uber H3 e Google S2.
  - [`engine_catalog_provenance.py`](../examples/engine_catalog_provenance.py) - Catálogo de fontes, cache Hive-Parquet e proveniência FAIR.
- 💛 [**Notebook Demonstrativo (Setembro Amarelo)**](https://colab.research.google.com/github/MarcelDevBr/brhealth/blob/main/examples/setembro_amarelo_perfil_epidemiologico.ipynb): Exemplo executável completo em Google Colab analisando o perfil epidemiológico de lesões autoprovocadas no Brasil.

</details>

<details open>
<summary><h3>🏥 2. Bioestatística, Epidemiologia e Economia da Saúde</h3></summary>

Fundamentação formal, padronizações sanitárias e formulações matemáticas em LaTeX:

- 📑 [**Manual Geral de Utilização e Métricas Científicas**](USAGE_GUIDE.md):
  - Lista Brasileira de Condições Sensíveis à Atenção Primária (**Portaria MS/SAS nº 221/2008**).
  - Formulação econométrica do **Retorno sobre o Investimento (ROI)** da Atenção Básica em custos hospitalares evitáveis.
  - Anos Potenciais de Vida Perdidos (**APVP / YLL**).
  - Taxa Padronizada Direta de Mortalidade com a **População Padrão Mundial da OMS (18 faixas etárias)**.
- 💰 [**Estudo Analítico de Custos Hospitalares de CSAP**](architecture/exemplo_analise_custos_csap.md): Aplicação em microdados reais da Morbidade Hospitalar do SUS (SIH/AIH).
- 🧬 [**Ontologias Médicas e Terminologias Clínicas**](PYTHON_GUIDE.md#7-ontologias-médicas-farmácia-e-validação-biológica):
  - Validação biológica estrita de patologias contra sexo e faixa etária.
  - Mapeamentos cruzados internacionais: CID-9 $\to$ CID-10 $\to$ CID-11 $\to$ SNOMED-CT.
  - Tabela Unificada de Procedimentos do SUS (**SIGTAP**) e Farmacologia (**ATC / RxNorm**).

</details>

<details open>
<summary><h3>🗺️ 3. Geoprocessamento, Harmonização Territorial e Grades Espaciais</h3></summary>

Integridade cadastral do Brasil e sistemas globais discretos:

- 🏛️ [**Harmonização Territorial do IBGE**](PYTHON_GUIDE.md#3-harmonização-e-validação-territorial-ibge):
  - Algoritmo de **Luhn Módulo 10** para validação e geração do 7º dígito verificador oficial.
  - Resolução de inconsistências entre cadastros legados de 6 dígitos e censos de 7 dígitos.
  - Reconciliação histórica territorial (1970–2026): transição constitucional de Goiás para o Tocantins, extinção de Fernando de Noronha e emancipações municipais.
- ⬢ [**Discrete Global Grid Systems (DGGS)**](PYTHON_GUIDE.md#6-geoprocessamento-e-indexação-espacial-uber-h3--google-s2):
  - Grade hexagonal **Uber H3** (resoluções 7, 8 e 9 para bairros e quarteirões).
  - Grade quadrilateral esférica **Google S2 Geometry**.
  - Operações de vizinhança topológica ($k$-ring disk) e detecção de aglomerados espaciais de arboviroses (Dengue, Chikungunya, Zika).

</details>

<details>
<summary><h3>💾 4. Decodificadores Nativos e Formatos Legados do DATASUS</h3></summary>

Alta performance sem binários legados externos:

- ⚙️ [**Descompressão Blast PKWARE DCL (.dbc)**](USAGE_GUIDE.md#3-decodificador-nativo-blast-dcl-dbc):
  - Implementação nativa e segura em Rust do algoritmo de descompressão Blast DCL do DATASUS.
  - Elimina dependências externas como `dbc2dbf` ou bibliotecas C legadas.
- 📊 [**Decodificação Colunar de Tabelas dBase (.dbf)**](USAGE_GUIDE.md#4-leitor-colunar-dbf-arrow):
  - Leitura direta para buffers contíguos de 64 bytes em `RecordBatch` Apache Arrow.
  - Tratamento nativo de acentuação e encodings ISO-8859-1 / CP850 / UTF-8.
- 🗄️ [**Camada de Armazenamento Hive-Parquet com Time-Travel**](architecture/sdd_brhealth.md):
  - Particionamento eficiente no formato Hive (`source=.../year=.../jurisdiction=...`).
  - Compressão colunar Snappy/ZSTD com suporte a snapshots históricos.

</details>

<details>
<summary><h3>🌐 5. Catálogo Federado de Fontes de Dados (26 Fontes)</h3></summary>

Conectividade transparente com bases de dados da saúde coletiva brasileira e global:

- 🇧🇷 **Ministério da Saúde / DATASUS**:
  - `datasus.sim` - Sistema de Informações sobre Mortalidade (Declarações de Óbito).
  - `datasus.sih` - Sistema de Informações Hospitalares (Autorizações de Internação Hospitalar - AIH).
  - `datasus.sinasc` - Sistema de Nascidos Vivos (Declarações de Nascido Vivo).
  - `datasus.sinan` - Agravos de Notificação Compulsória (Dengue, Tuberculose, Hanseníase, etc.).
  - `datasus.siasus` - Produção e Procedimentos Ambulatoriais.
  - `datasus.cnes` - Cadastro Nacional de Estabelecimentos de Saúde.
  - `datasus.sipni` - Programa Nacional de Imunizações (Vacinação).
  - `datasus.sisvan` - Vigilância Alimentar e Nutricional.
  - `datasus.siscan` - Vigilância do Câncer (Mamografia e Citopatologia).
  - `datasus.bps` - Banco de Preços em Saúde.
- 📊 **IBGE / Determinantes Sociais**:
  - `ibge.censo` - Microdados e Agregados dos Censos Demográficos.
  - `ibge.pnad` - Pesquisa Nacional por Amostra de Domicílios Contínua.
  - `ibge.pof` - Pesquisa de Orçamentos Familiares.
  - `ibge.pense` - Pesquisa Nacional de Saúde do Escolar.
  - `ibge.munic` - Pesquisa de Informações Básicas Municipais.
- 🌿 **Determinantes Ambientais e Climáticos**:
  - `environmental.inmet` - Estações Meteorológicas Convencionais e Automáticas.
  - `environmental.bdqueimadas` - Focos de Queimadas e Incêndios Florestais (INPE).
  - `environmental.prodes` - Desmatamento na Amazônia Legal e Cerrado (INPE).
  - `environmental.sisagua` - Vigilância da Qualidade da Água para Consumo Humano.
- 🤝 **Vulnerabilidade Social e Proteção Social**:
  - `mds.cadunico` - Cadastro Único para Programas Sociais do Governo Federal.
- 🌍 **Agências Internacionais e Globais**:
  - `global.copernicus_era5` - Reanálise Climática Global (Temperatura, Umidade, Radiação).
  - `global.who_gho` - World Health Organization Global Health Observatory.
  - `global.ihme_gbd` - Institute for Health Metrics and Evaluation - Global Burden of Disease.
  - `global.worldpop` - Distribuição e Densidade Espacial Populacional em Alta Resolução.
  - `global.openaq` - Rede Global de Monitoramento da Qualidade do Ar (PM2.5, PM10, NO2, O3).
  - `global.paho_plisa` - Plataforma de Informações de Saúde para as Américas (OPAS/OMS).
- 🔌 [**Guia para Extensão e Implementação de Novas Fontes (SPI)**](EXTENDING_BRHEALTH.md): Como adicionar novas fontes implementando a trait assíncrona `HealthDataSourceSPI`.

</details>

<details>
<summary><h3>🏛️ 6. Engenharia de Software, Arquitetura e Governança FAIR</h3></summary>

Rigor arquitetural, padrões de código e rastreabilidade científica:

- 🏗️ [**Software Design Document (SDD)**](architecture/sdd_brhealth.md): Especificação completa da Arquitetura Hexagonal Orientada a Dados (Hexagonal DOD), contratos Inbound/Outbound, alinhamento Arrow de 64 bytes e gestão de memória.
- 📜 [**Diretrizes e Regras do Projeto (AGENTS.md)**](../AGENTS.md): Política de Zero Unwraps em produção, Clippy estrito (`-D warnings`), cobertura por testes de propriedades (*proptest*) e licenciamento duplo AGPLv3.
- 🔒 [**Governança de Dados e Conformidade com a LGPD**](LGPD_GOVERNANCE.md): Protocolos de anonimização, pseudonimização, agregação territorial k-anônima e conformidade com a Lei Geral de Proteção de Dados (Lei nº 13.709/2018).
- 🌐 [**Linhagem Criptográfica e Padrões FAIR (W3C PROV-O)**](architecture/sdd_brhealth.md#manifesto-fair-e-grafo-w3c-prov-o): Geração de manifestos JSON-LD com hashes SHA-256 de dados brutos para total auditabilidade e reprodutibilidade científica.
- 🔄 [**CI/CD, Workflows e Automação**](CICD_AND_WORKFLOWS.md): Pipelines de testes automatizados, verificação multi-plataforma e empacotamento PyPI/Crates.io.
- 💡 [**Concepção e Visão do Projeto**](architecture/ideia.md): Manifesto de fundação do BRHealth.

</details>

---

## 🌐 Como Habilitar a Documentação com Menus no GitHub Pages (MkDocs Material)

O repositório já está estruturado para gerar automaticamente um portal estático com **menus retráteis, busca instantânea, abas de código e modo escuro** via [MkDocs Material](https://squidfunk.github.io/mkdocs-material/).

Para ativar a publicação automática:
1. No seu repositório no GitHub, acesse **Settings** > **Pages**.
2. Em **Build and deployment** > **Source**, selecione **GitHub Actions**.
3. O workflow [`.github/workflows/docs.yml`](../.github/workflows/docs.yml) publicará o site automaticamente a cada commit na branch `main`.

---

<div align="center">

**BRHealth** • *Aceleração Científica para a Saúde Pública Brasileira*  
Copyright (c) 2024-2026 Marcel &lt;MarcelDevBr&gt; and BRHealth Contributors.

</div>
