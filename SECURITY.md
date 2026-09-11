# Política de Segurança e Divulgação Responsável — BRHealth

## Reportando Vulnerabilidades de Segurança

A equipe do projeto **BRHealth** leva a sério a segurança de software, a integridade da cadeia de suprimentos e a proteção no parsing de arquivos binários legados (como o algoritmo PKWARE DCL / Blast do DATASUS).

Se você identificar uma vulnerabilidade de segurança (ex: estouro de buffer no decodificador binário, negação de serviço, falha de injeção ou dependência comprometida), **por favor, não abra uma Issue pública**.

Em vez disso, envie um e-mail com a descrição detalhada para:
- **E-mail de Segurança**: `marcel.dev.br@gmail.com` ou via mensagem privada criptografada para os mantenedores listados em `Cargo.toml`.

### Informações Desejadas no Relatório
1. **Descrição da Vulnerabilidade**: Impacto potencial e vetor de ataque.
2. **Passos para Reprodução**: Código de exemplo, arquivo `.dbc`/`.dbf` malformado de teste ou comando CLI.
3. **Versão Afetada**: Número da versão (ex: `0.1.0`) e commit SHA.

---

## Processo de Resposta

- **Confirmação**: Responderemos em até 48 horas úteis confirmando o recebimento do relatório.
- **Avaliação e Patch**: Trabalharemos em um patch de correção privado e disponibilizaremos um release de segurança no Cargo/PyPI.
- **Divulgação**: Após o lançamento da versão corrigida, creditaremos o pesquisador de segurança nas notas do release (salvo se o pesquisador preferir anonimato).

---

## Práticas de Segurança Ativas no BRHealth

- **Código Nativo Seguro**: Desenvolvido 100% em Rust seguro, eliminando bugs de corrupção de memória (*use-after-free*, *double-free*, *buffer overflow*) típicos de bibliotecas legadas em C/C++.
- **Zero Unwraps em Produção**: Propagação rígida de erros via `Result<T, PortError>`, impedindo panics não previstos.
- **Rede Criptografada**: Conexões com `rustls-tls` sem dependência de OpenSSL legado.
- **Verificação de Integridade Criptográfica**: Hash SHA-256 e proveniência auditável W3C PROV-O para todos os lotes colunar exportados.
