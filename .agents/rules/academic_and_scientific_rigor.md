# Regras de Rigor Acadêmico, Científico e Tecnológico

O BRHealth foi projetado para fundamentar pesquisas epidemiológicas em periódicos de primeira linha (*The Lancet*, *Nature Medicine*, *Cadernos de Saúde Pública*, *Revista de Saúde Pública*) e orientar políticas públicas de saúde com evidências incontestáveis.

---

## 1. Documentação Matemática Formal e Bioestatística
- **LaTeX Obrigatório nas Docstrings**: Todas as transformações, taxas brutas ou ajustadas, cálculos de indicadores (APVP, taxas padronizadas de mortalidade, CSAP, índices de segregação) devem conter a dedução matemática formal em blocos LaTeX na documentação da função.
- **Rastreabilidade de Fontes**: Todo algoritmo deve citar a norma, portaria ministerial ou artigo seminal em que se baseia (ex: *Portaria MS/SAS nº 221/2008* para CSAP; *Resolução IBGE / Metodologia DTB* para Módulo 10).

---

## 2. Princípios FAIR e W3C PROV-O
- **Reprodutibilidade Estrita**: Toda análise deve ser determinística. Para dados sujeitos a retificações governamentais posteriores, deve-se suportar o congelamento temporal via snapshots imutáveis (`as_of_snapshot`).
- **Hashes Criptográficos no Voo**: Os payloads brutos extraídos do DATASUS ou fontes internacionais devem ter seu hash SHA-256 computado durante o streaming e gravado no manifesto de proveniência.

---

## 3. Validação Contra Dados Oficiais
- Todo harmonizador de municípios, ontologias CID-10/CID-11 ou tabelas de procedimentos SIGTAP deve ser testado contra dados de referência canônicos fornecidos pelos órgãos reguladores (IBGE, DATASUS, OMS).
