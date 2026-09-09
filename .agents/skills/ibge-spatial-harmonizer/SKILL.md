---
name: ibge-spatial-harmonizer
description: >-
  Use this skill when implementing, testing or extending IBGE municipality code validation
  (6 to 7 digits Luhn Modulo 10 DV), territorial transition history (1970-2026), or discrete spatial
  indexing with Uber H3 and S2 Geometry.
---

# IBGE & Spatial Harmonizer Skill

Esta skill descreve as regras e formulações matemáticas para validação e reconciliação territorial dos municípios brasileiros e sua indexação espacial global.

## 1. Formulação Matemática Oficial do Dígito Verificador (IBGE Módulo 10)

Dado o vetor de dígitos do código municipal de 6 dígitos:
$$D = [d_1, d_2, d_3, d_4, d_5, d_6] \in \{0, \dots, 9\}^6$$
E o vetor de pesos oficiais alternados:
$$W = [1, 2, 1, 2, 1, 2]$$

Para cada posição $i \in \{1, \dots, 6\}$:
$$p_i = d_i \times w_i$$
$$s_i = \lfloor p_i / 10 \rfloor + (p_i \pmod{10})$$

Calcula-se a soma total dos dígitos dos produtos:
$$S = \sum_{i=1}^{6} s_i$$
O resto da divisão por 10:
$$R = S \pmod{10}$$
O Dígito Verificador (DV) é obtido por:
$$\text{DV} = (10 - R) \pmod{10}$$

### Casos Canônicos de Teste Oficial
- **São Paulo/SP** (`355030`): $S = 12 \implies R = 2 \implies \text{DV} = \mathbf{8}$ $\implies$ **`3550308`**
- **Rio de Janeiro/RJ** (`330455`): $S = 23 \implies R = 3 \implies \text{DV} = \mathbf{7}$ $\implies$ **`3304557`**
- **Belo Horizonte/MG** (`310620`): $S = 10 \implies R = 0 \implies \text{DV} = \mathbf{0}$ $\implies$ **`3106200`**
- **Campinas/SP** (`350950`): $S = 18 \implies R = 8 \implies \text{DV} = \mathbf{2}$ $\implies$ **`3509502`**
- **Salvador/BA** (`292740`): $S = 22 \implies R = 2 \implies \text{DV} = \mathbf{8}$ $\implies$ **`2927408`**

## 2. Testes de Propriedades com `proptest`
Além dos casos canônicos, todo gerador de DV do IBGE deve ser validado via property-based testing:
1. Para qualquer string de 6 dígitos numéricos, $\text{DV} \in \{0, \dots, 9\}$.
2. Concatenando o DV gerado, o código de 7 dígitos resultante é idempotente sob validação.
3. Para qualquer entrada com menos de 6, mais de 7 ou caracteres não-dígitos, a função retorna `Err(PortError::ValidationError(...))` de forma determinística.

## 3. Indexação Espacial H3/S2
- Centróides municipais e coordenadas de eventos são indexados em inteiros `uint64` H3 (resoluções 7 a 9).
- Spatial joins operam como joins colunares com chave inteira em tempo de consulta.
