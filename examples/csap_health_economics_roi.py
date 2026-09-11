# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Avaliação Econômica em Saúde Coletiva e Vigilância de CSAP:
1. Classificação de diagnósticos da CID-10 conforme a Portaria MS/SAS nº 221/2008 (19 Grupos Canônicos).
2. Simulação de coorte hospitalar de internações (SIH-SUS / AIH) com Polars e Apache Arrow.
3. Quantificação de Indicadores Epidemiológicos e de Gestão Hospitalar:
   - Taxa de Internações por CSAP por 10.000 habitantes:
     $$\\text{Taxa CSAP} = \\left( \\frac{\\sum N_{\\text{CSAP}}}{\\text{População}} \\right) \\times 10.000$$
   - Custo Hospitalar Total Evitável:
     $$\\text{Custo Evitável} = \\sum_{j \\in \\text{CSAP}} \\text{VAL\\_TOT}_j$$
   - Diárias Hospitalares Evitáveis (Dias de leito liberados para alta complexidade).
4. Modelagem Econométrica de Retorno sobre o Investimento (ROI) em Atenção Primária:
   $$\\text{ROI}_{\\text{APS}} = \\frac{(\\alpha \\cdot \\text{Custo Evitável}) - \\text{Investimento}_{\\text{APS}}}{\\text{Investimento}_{\\text{APS}}}$$
   onde $\\alpha \\in [0.0, 1.0]$ representa a Fração Atribuível Evitável na literatura.
"""

import warnings
import polars as pl
import brhealth
from brhealth import classify_cid10, csap_group_name, is_csap, compute_roi

# Suprimir avisos transitórios de migração de bibliotecas
warnings.filterwarnings("ignore", category=FutureWarning)


def main() -> None:
    print("=" * 75)
    print("  BRHealth: Avaliação Econômica e Vigilância de CSAP (Portaria 221/2008)")
    print("=" * 75)

    # 1. Demonstração de Classificação Ontológica Unitária
    print("\n[1] Classificação de Códigos CID-10 nos 19 Grupos Oficiais de CSAP:")
    amostra_cids = [
        ("J45.0", "Asma extrínseca"),
        ("I10", "Hipertensão essencial"),
        ("E11.9", "Diabetes mellitus tipo 2"),
        ("A09", "Gastroenterite e colite de origem infecciosa"),
        ("J14", "Pneumonia por Haemophilus influenzae"),
        ("I50.0", "Insuficiência cardíaca congestiva"),
        ("G40.9", "Epilepsia não especificada"),
        ("S06.0", "Concussão cerebral (Trauma / Não-CSAP)"),
        ("C50.9", "Neoplasia maligna da mama (Oncologia / Não-CSAP)"),
    ]

    for cid, descricao in amostra_cids:
        pertence = is_csap(cid)
        grupo_id = classify_cid10(cid)
        if pertence and grupo_id is not None:
            nome_grupo = csap_group_name(grupo_id)
            print(f"    - [{cid:>6}] CSAP Grupo {grupo_id:>2}: {nome_grupo:<35} | {descricao}")
        else:
            print(f"    - [{cid:>6}] NÃO-CSAP (Não sensível à APS)              | {descricao}")

    # 2. Simulação de Coorte Hospitalar Municipal (SIH-SUS)
    print("\n[2] Coorte Hospitalar Sintética (Simulando Internações AIH em Município de Médio Porte):")
    dados_aih = {
        "num_aih": [f"352410{i:06d}" for i in range(1, 11)],
        "primary_diagnosis": [
            "J450",  # G7: Asma
            "I10",   # G9: Hipertensão
            "E119",  # G13: Diabetes Mellitus
            "A09",   # G2: Gastroenterites
            "J14",   # G6: Pneumonias bacterianas
            "I500",  # G11: Insuficiência Cardíaca
            "S060",  # Não-CSAP (Trauma intracraniano)
            "C509",  # Não-CSAP (Neoplasia de mama)
            "K358",  # Não-CSAP (Apendicite aguda)
            "E109",  # G13: Diabetes Mellitus tipo 1
        ],
        "total_cost": [
            480.0,    # Asma
            350.0,    # Hipertensão
            920.0,    # Diabetes
            260.0,    # Gastroenterite
            1150.0,   # Pneumonia
            2400.0,   # Insuficiência Cardíaca
            8500.0,   # Trauma
            14200.0,  # Câncer
            3800.0,   # Apendicite
            1100.0,   # Diabetes
        ],
        "length_of_stay": [2, 1, 4, 2, 5, 7, 12, 18, 5, 4],
    }

    df = pl.DataFrame(dados_aih)

    # Enriquecimento com regras analíticas do BRHealth
    grupos_ids = [classify_cid10(c) for c in df["primary_diagnosis"]]
    flags_csap = [is_csap(c) for c in df["primary_diagnosis"]]
    nomes_grupos = [csap_group_name(g) if g else "Não-CSAP" for g in grupos_ids]

    df = df.with_columns([
        pl.Series("is_csap", flags_csap),
        pl.Series("csap_group_id", grupos_ids),
        pl.Series("csap_group_name", nomes_grupos),
    ])

    print(df.select([
        "num_aih", "primary_diagnosis", "is_csap", "csap_group_name", "total_cost", "length_of_stay"
    ]))

    # 3. Consolidação de Métricas Epidemiológicas
    populacao_municipio = 40_000
    total_internacoes = df.height
    csap_internacoes = df.filter(pl.col("is_csap")).height
    proporcao_csap = csap_internacoes / total_internacoes
    taxa_csap_10k = (csap_internacoes / populacao_municipio) * 10_000

    custo_total = df["total_cost"].sum()
    custo_evitavel = df.filter(pl.col("is_csap"))["total_cost"].sum()
    prop_custo_evitavel = (custo_evitavel / custo_total) * 100

    diarias_totais = df["length_of_stay"].sum()
    diarias_evitaveis = df.filter(pl.col("is_csap"))["length_of_stay"].sum()

    print("\n[3] Painel de Indicadores de Gestão e Economia da Saúde:")
    print(f"    - População Municipal de Referência:      {populacao_municipio:,} hab.")
    print(f"    - Total de Internações Analisadas:       {total_internacoes}")
    print(f"    - Internações por CSAP (Evitáveis na APS): {csap_internacoes} ({proporcao_csap:.1%})")
    print(f"    - Taxa Bruta de CSAP por 10.000 hab:       {taxa_csap_10k:.2f}")
    print(f"    - Custo Hospitalar Total:                R$ {custo_total:>10,.2f}")
    print(f"    - Custo Hospitalar Evitável (CSAP):       R$ {custo_evitavel:>10,.2f} ({prop_custo_evitavel:.1f}%)")
    print(f"    - Diárias de Internação Totais:          {diarias_totais} dias")
    print(f"    - Diárias Hospitalares Evitáveis:        {diarias_evitaveis} dias leito")

    # 4. Agrupamento por Categoria de Condição Sensível
    print("\n[4] Detalhamento por Grupo de CSAP:")
    resumo_grupos = (
        df.filter(pl.col("is_csap"))
        .group_by(["csap_group_id", "csap_group_name"])
        .agg([
            pl.len().alias("casos"),
            pl.col("total_cost").sum().alias("custo_total"),
            pl.col("length_of_stay").sum().alias("diarias"),
        ])
        .sort("custo_total", descending=True)
    )
    print(resumo_grupos)

    # 5. Modelagem Econométrica de Retorno sobre o Investimento (ROI)
    print("\n[5] Modelagem de Retorno sobre Investimento (ROI) com Fortalecimento da Atenção Básica:")
    # Cenário de Política Pública:
    # Contratação e qualificação de equipe multiprofissional da ESF para controle de crônicos (HAS e Diabetes)
    investimento_aps = 1_500.0  # Investimento incremental na Atenção Primária
    fracao_atribuivel = 0.45    # Redução plausível de 45% nas internações de crônicos (alpha = 0.45)

    roi_estimado = compute_roi(
        avoidable_cost=custo_evitavel,
        investment=investimento_aps,
        attributable_fraction=fracao_atribuivel,
    )

    economia_esperada = custo_evitavel * fracao_atribuivel
    retorno_liquido = economia_esperada - investimento_aps

    print(f"    - Investimento Alocado na APS (UBS/eSF):  R$ {investimento_aps:>10,.2f}")
    print(f"    - Fração Atribuível de Eficácia (alpha):   {fracao_atribuivel:>10.0%}")
    print(f"    - Economia Hospitalar Esperada (Bruta):   R$ {economia_esperada:>10,.2f}")
    print(f"    - Retorno Líquido para o Fundo Municipal: R$ {retorno_liquido:>10,.2f}")
    print(f"    - ROI Calculado (Fórmula Oficial):         {roi_estimado * 100:>9.1f}%")
    print(f"      (Cada R$ 1,00 investido na APS gera R$ {1.0 + roi_estimado:.2f} em benefícios econômico-sanitários)")

    print("\n" + "=" * 75)
    print("  Avaliação de CSAP e ROI concluída com sucesso!")
    print("=" * 75)


if __name__ == "__main__":
    main()
