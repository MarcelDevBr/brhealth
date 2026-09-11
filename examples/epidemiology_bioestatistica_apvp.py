# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Bioestatística, Vigilância Epidemiológica e Ontologias Médicas:
1. Anos Potenciais de Vida Perdidos (APVP / YLL - Years of Life Lost):
   $$\\text{APVP} = \\sum_{i=1}^{n} d_i \\cdot (L - a_i), \\quad d_i = 1 \\text{ se } a_i < L$$
   onde $L$ é a idade limite (70 anos pelo Ministério da Saúde, ou 75 anos pela OMS).
2. Taxa Padronizada de APVP por 100.000 habitantes:
   $$\\text{Taxa APVP} = \\left( \\frac{\\text{APVP Total}}{\\text{População sob Risco}} \\right) \\times 100.000$$
3. Padronização Direta de Mortalidade (População Padrão Mundial da OMS, 18 faixas etárias):
   $$\\text{TME}_{\\text{OMS}} = \\sum_{k=1}^{18} w_k \\cdot \\left( \\frac{D_k}{P_k} \\right) \\times 100.000$$
4. Validação Biológica Estrita de Microdados (inconsistências de sexo e idade).
5. Mapeamento Ontológico Clínico: CID-9 $\\to$ CID-10 $\\to$ CID-11 $\\to$ SNOMED-CT.
6. Farmacoepidemiologia (ATC / RxNorm) e Tabela Unificada de Procedimentos do SUS (SIGTAP).
"""

import brhealth
from brhealth import (
    compute_apvp,
    compute_apvp_rate,
    compute_age_standardized_mortality_rate,
    validate_biological_consistency,
    icd10_chapter,
    map_icd9_to_icd10,
    map_icd10_to_icd11,
    map_icd10_to_snomed,
    lookup_atc,
    map_atc_to_rxnorm,
    is_amputation_procedure,
    is_dialysis_procedure,
    parse_sigtap_code,
)


def main() -> None:
    print("=" * 80)
    print("  BRHealth: Bioestatística, Mortalidade Prematura (APVP) e Ontologias Médicas")
    print("=" * 80)

    # 1. Anos Potenciais de Vida Perdidos (APVP / YLL)
    print("\n[1] Análise de Mortalidade Prematura - APVP (Ministério da Saúde):")
    idades_obito = [14, 22, 35, 48, 62, 68, 73, 81, 89]
    limite_ms = 70  # Padrão canônico do Ministério da Saúde do Brasil
    limite_oms = 75 # Padrão internacional da OMS

    apvp_ms = compute_apvp(idades_obito, cutoff_age=limite_ms)
    apvp_oms = compute_apvp(idades_obito, cutoff_age=limite_oms)

    populacao_alvo = 50_000
    taxa_apvp_ms = compute_apvp_rate(total_apvp=apvp_ms, population=populacao_alvo)
    taxa_apvp_oms = compute_apvp_rate(total_apvp=apvp_oms, population=populacao_alvo)

    print(f"    - Coorte de Óbitos: {idades_obito}")
    print(f"    - População Municipal sob Risco: {populacao_alvo:,} hab.")
    print(f"    - Limite MS  (L = {limite_ms} anos): {apvp_ms:>4} anos perdidos | Taxa: {taxa_apvp_ms:>6.2f} / 100k hab.")
    print(f"    - Limite OMS (L = {limite_oms} anos): {apvp_oms:>4} anos perdidos | Taxa: {taxa_apvp_oms:>6.2f} / 100k hab.")

    # 2. Padronização Direta de Mortalidade (População Mundial da OMS)
    print("\n[2] Padronização Direta da Taxa de Mortalidade (18 Faixas Quinquenais OMS):")
    # 18 faixas etárias: 0-4, 5-9, 10-14, 15-19, ..., 80-84, 85+ anos
    obitos_observados = [12, 3, 2, 5, 8, 11, 15, 22, 34, 49, 70, 95, 130, 165, 210, 240, 190, 140]
    populacao_local = [4_500, 4_800, 5_000, 5_200, 5_500, 5_400, 5_200, 4_900,
                       4_600, 4_200, 3_800, 3_400, 2_900, 2_300, 1_700, 1_100, 650, 350]

    taxa_bruta = (sum(obitos_observados) / sum(populacao_local)) * 100_000
    taxa_padronizada = compute_age_standardized_mortality_rate(
        observed_deaths=obitos_observados,
        local_pop=populacao_local,
    )

    print(f"    - Total de Óbitos na Coorte:       {sum(obitos_observados):,}")
    print(f"    - População Total do Município:    {sum(populacao_local):,} hab.")
    print(f"    - Taxa Bruta de Mortalidade:       {taxa_bruta:>8.2f} por 100.000 hab.")
    print(f"    - Taxa Padronizada Direta (OMS):   {taxa_padronizada:>8.2f} por 100.000 hab.")
    print(f"      (A padronização elimina o viés de envelhecimento populacional entre territórios)")

    # 3. Validação de Consistência Biológica Estrita
    print("\n[3] Validação de Consistência Biológica de Eventos em Saúde:")
    casos_validacao = [
        ("O00", "F", 28, "Gravidez ectópica em mulher fértil"),
        ("O00", "M", 28, "Gravidez ectópica em indivíduo do sexo masculino"),
        ("C61", "M", 68, "Neoplasia maligna de próstata em homem idoso"),
        ("C61", "F", 68, "Neoplasia maligna de próstata em mulher"),
        ("G30", "F", 82, "Doença de Alzheimer em mulher idosa"),
        ("G30", "M", 2,  "Doença de Alzheimer em criança de 2 anos"),
    ]

    for cid, sexo, idade, descricao in casos_validacao:
        try:
            valido = validate_biological_consistency(icd10=cid, sex=sexo, age_years=idade)
            status = "✓ VÁLIDO    " if valido else "✗ INCONSISTENTE"
            msg = ""
        except Exception as err:
            status = "✗ REJEITADO "
            msg = f"-> Erro: {err}"
        print(f"    - [{status}] CID {cid:<4} | Sexo: {sexo} | Idade: {idade:>2}a | {descricao} {msg}")

    # 4. Ontologias Médicas e Mapeamentos Cruzados
    print("\n[4] Interoperabilidade Semântica e Mapeamento de Ontologias Clínicas:")
    print("    a) CID-9 -> CID-10 (Séries temporais longas pré e pós-1996):")
    for cid9 in ["493", "410", "250"]:
        cid10 = map_icd9_to_icd10(cid9)
        print(f"       - CID-9: {cid9:<5} -> CID-10: {cid10}")

    print("\n    b) CID-10 -> CID-11 (Transição sanitária da OMS):")
    for cid10 in ["I10", "E11", "J45"]:
        cid11 = map_icd10_to_icd11(cid10)
        print(f"       - CID-10: {cid10:<5} -> CID-11: {cid11}")

    print("\n    c) CID-10 -> SNOMED-CT (Terminologia clínica para prontuários eletrônicos):")
    for cid10 in ["I10", "E11", "J45"]:
        snomed = map_icd10_to_snomed(cid10)
        print(f"       - CID-10: {cid10:<5} -> Concept ID SNOMED-CT: {snomed}")

    print("\n    d) Metadados Oficiais dos Capítulos da CID-10:")
    for cod in ["I21", "C50", "A09", "V01"]:
        cap = icd10_chapter(cod)
        print(f"       - [{cod:<3}] Capítulo {cap['roman']:<4} ({cap['number']:>2}): {cap['title_pt']}")

    # 5. Farmacologia (ATC / RxNorm) e Procedimentos SIGTAP do SUS
    print("\n[5] Farmacoepidemiologia (ATC / RxNorm) e Procedimentos Hospitalares (SIGTAP):")
    medicamentos_atc = [
        ("A10BA02", "Hipoglicemiante oral (Diabetes)"),
        ("C09AA02", "Inibidor da ECA (Anti-hipertensivo)"),
        ("J01CA04", "Antibacteriano sistêmico (Amoxicilina)"),
    ]
    for atc, ind in medicamentos_atc:
        principio = lookup_atc(atc)
        rxnorm = map_atc_to_rxnorm(atc)
        print(f"    - ATC: {atc} ({ind})")
        print(f"      Princípio Ativo: {principio} | RxNorm Concept: {rxnorm}")

    print("\n    Procedimentos Hospitalares Estruturados (SIGTAP):")
    procedimentos = [
        ("0407040101", "Amputação / desarticulação de membro"),
        ("0305010107", "Hemodiálise crônica (máximo 3 sessões por semana)"),
        ("0301010072", "Consulta médica em atenção primária"),
    ]
    for proc, desc in procedimentos:
        e_amputacao = is_amputation_procedure(proc)
        e_dialise = is_dialysis_procedure(proc)
        g, subg, forma, seq, dv = parse_sigtap_code(proc)

        classificacao = []
        if e_amputacao:
            classificacao.append("Amputação")
        if e_dialise:
            classificacao.append("Terapia Renal Substitutiva")
        tag = f"[{', '.join(classificacao)}]" if classificacao else "[Geral]"

        print(f"    - Código SIGTAP: {proc} {tag:<28}")
        print(f"      Descrição: {desc}")
        print(f"      Estrutura: Grupo={g}, Subgrupo={subg:02d}, Forma={forma:02d}, Seq={seq:04d}, DV={dv}")

    print("\n" + "=" * 80)
    print("  Demonstração de Bioestatística e Ontologias concluída com sucesso!")
    print("=" * 80)


if __name__ == "__main__":
    main()
