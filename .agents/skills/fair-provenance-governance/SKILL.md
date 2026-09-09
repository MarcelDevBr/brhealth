---
name: fair-provenance-governance
description: >-
  Use this skill when implementing, generating or validating FAIR data governance manifests,
  W3C PROV-O cryptographic provenance graphs, SHA-256 stream hashing, or scientific reproducibility
  protocols.
---

# FAIR & Provenance Governance Skill

Esta skill estabelece o protocolo de auditoria e validação de reprodutibilidade científica segundo os princípios **FAIR** (*Findable, Accessible, Interoperable, Reusable*) e a ontologia **W3C PROV-O** para publicações em periódicos de impacto internacional (*The Lancet*, *Nature*, etc.).

## 1. Princípios FAIR no BRHealth
- **Findable**: Metadados estruturados em JSON-LD com UUIDv4 único por execução analítica.
- **Accessible**: Protocolos abertos (FTP streaming resiliente, HTTP REST) e documentação aberta.
- **Interoperable**: Schemas Apache Arrow alinhados com ontologias médicas universais (CID-10, CID-11, SNOMED-CT, SIGTAP, ISO 3166-1/2).
- **Reusable**: Licenciamento transparente (AGPLv3 / Modelo Comercial MarcelDevBr), citação permanente (`CITATION.cff` com DOI) e grafo completo de transformações.

## 2. Checklist de Auditoria Científica para Submissão de Artigos
Antes de submeter achados para publicação, o pesquisador/agente deve verificar:
1. [ ] Todos os dados brutos possuem hash SHA-256 computado no voo e gravado no manifesto?
2. [ ] O carimbo temporal UTC da extração e a versão dos servidores governamentais estão registrados?
3. [ ] A versão do motor `brhealth-core` e o commit hash do Git estão documentados?
4. [ ] O manifesto JSON pode ser validado contra o esquema oficial `fair-manifest.json`?
5. [ ] O time-travel determinístico (`as_of_snapshot`) gera datasets identicamente reproduzíveis bit a bit?

## 3. Streaming Hashing (SHA-256 no Voo)
O cálculo do hash SHA-256 nunca deve ser postergado ou exigir regravações de arquivos intermediários:
```rust
use sha2::{Digest, Sha256};

let mut hasher = Sha256::new();
// Atualiza a cada chunk recebido via rede
hasher.update(&chunk);
let result = format!("{:x}", hasher.finalize());
```
