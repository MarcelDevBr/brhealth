# Governança de Dados e Conformidade LGPD — BRHealth

**Projeto:** BRHealth (`MarcelDevBr/brhealth`)  
**Licença:** GNU Affero General Public License v3 (AGPLv3) / Duplo Licenciamento Comercial  
**Escopo:** Tratamento de Microdados Epidemiológicos, Demográficos e Determinantes Sociais da Saúde no Brasil.

---

## 1. Princípios Gerais e Enquadramento Legal

O motor analítico **BRHealth** foi concebido sob os princípios de **Privacy by Design** e **Security by Default**, alinhando-se integralmente à **Lei Geral de Proteção de Dados Pessoais (LGPD — Lei nº 13.709/2018)** e às normativas do Conselho Nacional de Saúde (CNS nº 466/2012 e nº 510/2016).

Os dados processados pelas fontes integradas (ex: DATASUS SIM, SINASC, SIH-SUS, CadÚnico, Censo IBGE) enquadram-se nas seguintes categorias:

1. **Dados Anonimizados e Microdados Públicos de Acesso Aberto** (Art. 12 da LGPD):
   - Microdados divulgados pelo Ministério da Saúde (DATASUS) e IBGE são submetidos a processos prévios de desidentificação pelo órgão custodiante.
   - O BRHealth opera sobre dados tabulares agregados em memória contígua (Apache Arrow) e índices espaciais discretos (Uber H3 / Google S2).

2. **Salvaguardas de Pesquisa em Saúde Pública** (Art. 7º, Inciso IX e Art. 13 da LGPD):
   - O tratamento de dados para realização de estudos por órgão de pesquisa garante, sempre que possível, a anonimização dos dados pessoais de saúde sensíveis.

---

## 2. Salvaguardas Técnicas Implementadas no BRHealth

### 2.1 Análise Territorial Agregada e Índice H3/S2
- Para mitigar riscos de reidentificação geográfica, o motor BRHealth suporta agregação espacial discreta usando resolução H3 configurável.
- Resoluções de alta granularidade (níveis H3 > 9) devem ser utilizadas com cautela em territórios de baixa densidade populacional para evitar triangulação de residências.

### 2.2 Reconciliação Histórica IBGE sem Rastros Individuais
- As rotinas de harmonização municipal (`harmonize_ibge_code`, `reconcile_historical_ibge_code`) operam estritamente sobre a malha territorial e códigos de municípios (Luhn Modulo 10 DV), sem correlacionar dados identificáveis de indivíduos.

### 2.3 Rastreabilidade Criptográfica e Auditabilidade FAIR (PROV-O)
- Todo processamento de lote colunar gera um manifesto **W3C PROV-O** assinado criptograficamente com hashes **SHA-256** dos buffers brutos e carimbos UTC.
- Isso assegura a reprodutibilidade científica sem expor conteúdos de identificação pessoal.

---

## 3. Recomendações para Pesquisadores e Instituições Usuárias

Ao utilizar o pacote Python ou CLI do BRHealth em ambiente corporativo, acadêmico ou hospitalar:

1. **Não realizar reidentificação cruzada**: É expressamente vedada qualquer tentativa de cruzar microdados públicos do BRHealth com bases externas visando identificar indivíduos ou famílias.
2. **Armazenamento Seguro de Enclaves Analíticos**: Caso os dados exportados em formato Parquet sejam armazenados em nuvem (ex: GCS, AWS S3), aplique criptografia em repouso (CMEK) e controle de acesso estrito (IAM).
3. **Relatórios Epidemiológicos Agregados**: Ao publicar mapas ou gráficos contendo contagens reduzidas de eventos raros (ex: óbitos por causas estigmatizantes com contagem $< 5$ em determinado município), aplique supressão de células pequenas conforme boas práticas da OMS/IBGE.
