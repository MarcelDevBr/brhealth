---
name: csap-health-economics
description: >-
  Use this skill when implementing health economics models, Ambulatory Care Sensitive Conditions
  (CSAP - Portaria MS/SAS 221/2008), avoidable hospitalization cost calculations, or Primary Care
  ROI evaluations in BRHealth.
---

# CSAP & Health Economics Skill

Esta skill descreve as formulações bioestatísticas e os modelos econométricos para a análise de **Condições Sensíveis à Atenção Primária (CSAP)** no SUS, em estrita conformidade com a **Portaria MS/SAS nº 221/2008**.

## 1. Formulação Bioestatística das Taxas de CSAP

### Taxa Bruta de Internação por CSAP
Para um determinado município ou região $m$ no ano $t$:
$$\text{Taxa Bruta CSAP}_{m,t} = \left( \frac{\sum_{i \in \text{CSAP}} N_{i,m,t}}{\text{População Total}_{m,t}} \right) \times 10.000\text{ hab.}$$

Onde $N_{i,m,t}$ representa o número de internações hospitalares (AIH pagas) cujo diagnóstico principal (`DIAG_PRINC`) pertence à lista da Portaria MS/SAS nº 221/2008.

### Taxa Padronizada por Idade e Sexo (Método Direto)
Considerando a população padrão de referência $P_{\text{ref}}$ (ex: Censo Demográfico do Brasil 2022) dividida em estratos etários $k$:
$$\text{Taxa Padronizada CSAP}_{m,t} = \sum_{k} \left( \frac{N_{k,m,t}}{P_{k,m,t}} \right) \times \frac{P_{k,\text{ref}}}{\sum_j P_{j,\text{ref}}} \times 10.000\text{ hab.}$$

## 2. Modelagem de Custos Evitáveis e Retorno sobre o Investimento (ROI)

### Custo Hospitalar Direto Evitável
$$\text{Custo Evitável}_{m,t} = \sum_{j \in \text{AIH}_{\text{CSAP}}} \text{VAL\_TOT}_j$$

### Modelo de Economia Líquida da Intervenção na Atenção Primária
Seja $C_{\text{UBS}}$ o custo anual de acompanhamento e medicamentos contínuos por paciente na atenção básica (aproximadamente R$ 600 a R$ 900/ano):
$$\Delta_{\text{Fin}} = \left( \alpha \times \text{Custo Evitável} \right) - \left( N_{\text{pacientes}} \times C_{\text{UBS}} \right)$$
Onde $\alpha$ é a fração atribuível evitável (tipicamente 0,30 a 0,50 segundo estudos epidemiológicos do Ministério da Saúde e Fiocruz).

## 3. Classificação Canônica da Portaria 221/2008 (19 Grupos)
- CIDs e regras de inclusão/exclusão (ex: exclusão de procedimentos cirúrgicos congênitos em certos grupos) devem ser rigorosamente validados contra a tabela oficial do DATASUS.
