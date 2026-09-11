# Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
# Licensed under the GNU Affero General Public License v3 (AGPLv3)
# or a commercial license agreement directly with the author.

"""
Exemplo de Geoprocessamento, Harmonização Territorial (IBGE) e Indexação Espacial:
1. Harmonização e Validação Territorial (IBGE):
   - Validação algorítmica de municípios pelo algoritmo de Luhn Módulo 10:
     $$s = \\sum_{i=1}^{6} \\left( \\lfloor (w_i \\cdot c_i) / 10 \\rfloor + ((w_i \\cdot c_i) \\bmod 10) \\right), \\quad w = (1, 2, 1, 2, 1, 2)$$
     $$DV = (10 - (s \\bmod 10)) \\bmod 10$$
   - Harmonização de 6 para 7 dígitos canônicos (resolvendo divergências entre SUS e IBGE).
2. Reconciliação Histórica de Malhas Municipais (1970–2026):
   - Tratamento de transições territoriais constitucionais (ex: cisão GO -> TO em 1988, Fernando de Noronha em 1988).
3. Discrete Global Grid Systems (DGGS) - Uber H3:
   - Conversão de coordenadas (latitude, longitude) para hexágonos H3 de 64 bits (`uint64`).
   - Resolução espacial e centróides.
   - Anéis topológicos de vizinhança ($k$-ring) e distâncias na grade hexagonal.
4. Google S2 Spherical Cell ID (64 bits):
   - Projeção esférica e hierarquia de células quadrilaterais.
5. Agregação e Clusterização Geoespacial de Casos Epidemiológicos com Polars.
"""

import warnings
import polars as pl
import brhealth
from brhealth import (
    calculate_ibge_dv,
    validate_ibge_code,
    harmonize_ibge_code,
    reconcile_historical_ibge_code,
    latlng_to_h3,
    h3_to_latlng,
    h3_grid_disk,
    h3_grid_distance,
    coord_to_s2_cell,
    s2_cell_to_coord,
)

warnings.filterwarnings("ignore", category=FutureWarning)


def main() -> None:
    print("=" * 80)
    print("  BRHealth: Geoprocessamento, Harmonização IBGE e Grades Espaciais (H3 & S2)")
    print("=" * 80)

    # 1. Harmonização e Validação do Dígito Verificador IBGE
    print("\n[1] Validação Territorial IBGE (Luhn Módulo 10):")
    municipios_teste = [
        ("355030", "São Paulo / SP"),
        ("330455", "Rio de Janeiro / RJ"),
        ("310620", "Belo Horizonte / MG"),
        ("530010", "Brasília / DF"),
        ("120040", "Rio Branco / AC"),
        ("3550308", "São Paulo / SP (já com 7 dígitos válidos)"),
        ("3550309", "Código com DV adulterado (deveria ser 8)"),
    ]

    for raw, nome in municipios_teste:
        valido = validate_ibge_code(raw)
        if len(str(raw).strip()) == 6:
            dv = calculate_ibge_dv(raw)
            canonico = harmonize_ibge_code(raw)
            print(f"    - [{raw}] {nome:<28} -> DV Calculado: {dv} | Canônico 7D: {canonico} | Válido: {valido}")
        else:
            print(f"    - [{raw}] {nome:<28} -> Validação Direta: {valido}")

    # 2. Reconciliação Histórica de Municípios Brasileiros (1970 - 2026)
    print("\n[2] Reconciliação Histórica Territorial (Séries Temporais Longas):")
    casos_historicos = [
        ("200001", 1980, "Território Federal de Fernando de Noronha (incorporado a PE na CF/88)"),
        ("520210", 1985, "Araguaína (antigo Goiás, emancipado como Tocantins em 1988)"),
        ("521150", 1986, "Miracema do Norte (antiga capital provisória do TO)"),
    ]

    for cod_antigo, ano_ref, descricao in casos_historicos:
        reconciliado = reconcile_historical_ibge_code(cod_antigo, reference_year=ano_ref)
        print(f"    - Código Histórico: {cod_antigo} (Ano Ref: {ano_ref})")
        print(f"      Contexto: {descricao}")
        print(f"      Código IBGE Contemporâneo: {reconciliado}\n")

    # 3. Malha Hexagonal Discreta Uber H3
    print("[3] Indexação Espacial em Malha Hexagonal Uber H3:")
    # Coordenadas de capitais e pontos focais
    locais = [
        ("Praça da Sé (São Paulo/SP)", -23.550520, -46.633308),
        ("Cristo Redentor (Rio de Janeiro/RJ)", -22.951916, -43.210487),
        ("Praça dos Três Poderes (Brasília/DF)", -15.800514, -47.864471),
        ("Marco Zero de Recife (Recife/PE)", -8.063169, -34.871139),
    ]

    resolucao_h3 = 8  # Resolução 8: hexágonos com área média ~0.74 km² (ideal para bairros/UBS)

    h3_indices = []
    for nome, lat, lon in locais:
        h3_idx = latlng_to_h3(lat, lon, resolution=resolucao_h3)
        c_lat, c_lon = h3_to_latlng(h3_idx)
        h3_indices.append(h3_idx)
        print(f"    - {nome}:")
        print(f"      Coords: ({lat:.6f}, {lon:.6f})")
        print(f"      H3 Index (Res {resolucao_h3}): {hex(h3_idx)} (uint64: {h3_idx})")
        print(f"      Centróide Reconstruído: ({c_lat:.6f}, {c_lon:.6f})")

    # Anel de Vizinhança Espacial (k-ring disk)
    sp_h3 = h3_indices[0]
    vizinhanca_k1 = h3_grid_disk(sp_h3, k=1)
    print(f"\n    Topologia Hexagonal:")
    print(f"    - Anel de vizinhança k=1 ao redor da Sé/SP: {len(vizinhanca_k1)} células (1 centro + 6 adjacentes)")
    dist_mesmo = h3_grid_distance(sp_h3, sp_h3)
    dist_vizinho = h3_grid_distance(sp_h3, vizinhanca_k1[1])
    print(f"    - Distância topológica central: {dist_mesmo} passos")
    print(f"    - Distância topológica para vizinho adjacente: {dist_vizinho} passo")

    # 4. Google S2 Geometry Spherical Cell ID
    print("\n[4] Indexação Esférica Google S2 Geometry:")
    nivel_s2 = 12  # Nível 12 S2: células com ~3 a 5 km²
    for nome, lat, lon in locais[:2]:
        s2_cell = coord_to_s2_cell(lat, lon, level=nivel_s2)
        r_lat, r_lon = s2_cell_to_coord(s2_cell)
        print(f"    - {nome}:")
        print(f"      S2 Cell ID (Level {nivel_s2}): {s2_cell} ({hex(s2_cell)})")
        print(f"      Reversão S2 -> Coords: ({r_lat:.6f}, {r_lon:.6f})")

    # 5. Pipeline Analítico: Agrupamento Geoespacial de Notificações com Polars
    print("\n[5] Análise de Agrupamento Espacial de Casos Epidemiológicos (Dengue / Arboviroses):")
    # Simulação de casos georreferenciados na Região Central de São Paulo
    pontos_dengue = [
        {"id": 1, "lat": -23.5505, "lon": -46.6333, "gravidade": "Grave"},
        {"id": 2, "lat": -23.5510, "lon": -46.6340, "gravidade": "Moderado"},
        {"id": 3, "lat": -23.5498, "lon": -46.6325, "gravidade": "Leve"},
        {"id": 4, "lat": -23.5600, "lon": -46.6500, "gravidade": "Leve"},
        {"id": 5, "lat": -23.5610, "lon": -46.6512, "gravidade": "Grave"},
        {"id": 6, "lat": -23.5620, "lon": -46.6495, "gravidade": "Moderado"},
        {"id": 7, "lat": -23.5800, "lon": -46.6800, "gravidade": "Leve"},
    ]

    df_dengue = pl.DataFrame(pontos_dengue)

    # Atribuir célula H3 (Resolução 9 ~ 0.1 km², precisão de quarteirão)
    res_cluster = 9
    h3_cells = [
        latlng_to_h3(row["lat"], row["lon"], resolution=res_cluster)
        for row in df_dengue.iter_rows(named=True)
    ]
    df_dengue = df_dengue.with_columns(pl.Series("h3_index", [hex(h) for h in h3_cells]))

    print("    Casos notificados com célula H3 associada:")
    print(df_dengue)

    # Agregação colunar rápida para detecção de clusters espaciais
    cluster_summary = (
        df_dengue.group_by("h3_index")
        .agg([
            pl.len().alias("total_casos"),
            pl.col("gravidade").filter(pl.col("gravidade") == "Grave").len().alias("casos_graves"),
        ])
        .sort("total_casos", descending=True)
    )
    print("\n    Clusters Espaciais Identificados (Hotspots Hexagonais):")
    print(cluster_summary)

    print("\n" + "=" * 80)
    print("  Geoprocessamento e Harmonização Territorial concluídos com sucesso!")
    print("=" * 80)


if __name__ == "__main__":
    main()
