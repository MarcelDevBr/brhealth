// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

#pragma once

/**
 * @file brhealth.hpp
 * @brief Modern C++20 RAII Bindings for the BRHealth Columnar Analytics Engine.
 */

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>

extern "C" {
    const char* brhealth_version();
    int32_t brhealth_calculate_ibge_dv(const char* ibge_6digits, uint8_t* out_dv);
    int32_t brhealth_coord_to_h3(double lat, double lon, uint8_t resolution, uint64_t* out_h3);
    int32_t brhealth_classify_csap(const char* cid10, uint8_t* out_group_id);
    int32_t brhealth_compute_primary_care_roi(
        double avoidable_cost,
        double investment,
        double attributable_fraction,
        double* out_roi
    );
}

namespace brhealth {

/**
 * @brief Exceção específica lançada por falhas na camada nativa do BRHealth.
 */
class EngineException : public std::runtime_error {
public:
    explicit EngineException(const std::string& message) : std::runtime_error(message) {}
};

/**
 * @brief Retorna a versão canônica compilada da biblioteca nativa BRHealth.
 */
inline std::string_view version() noexcept {
    const char* ver = brhealth_version();
    return (ver != nullptr) ? std::string_view(ver) : std::string_view("");
}

/**
 * @brief Calcula o Dígito Verificador (DV) oficial do IBGE pelo algoritmo de Luhn Módulo 10.
 *
 * @param code_6digits Código municipal do IBGE com exatamente 6 dígitos.
 * @return uint8_t Dígito Verificador (0 a 9).
 * @throws EngineException se o código for inválido.
 */
inline uint8_t calculate_ibge_dv(std::string_view code_6digits) {
    std::string null_terminated(code_6digits);
    uint8_t out_dv = 0;
    int32_t status = brhealth_calculate_ibge_dv(null_terminated.c_str(), &out_dv);
    if (status != 0) {
        throw EngineException("Falha ao calcular DV do IBGE para o código: " + null_terminated);
    }
    return out_dv;
}

/**
 * @brief Converte coordenadas geodésicas (WGS84) em um índice discreto hexagonal Uber H3.
 *
 * @param lat Latitude (-90.0 a 90.0).
 * @param lon Longitude (-180.0 a 180.0).
 * @param resolution Resolução H3 (0 a 15).
 * @return uint64_t Índice H3 canônico de 64 bits.
 * @throws EngineException se as coordenadas ou resolução forem inválidas.
 */
inline uint64_t latlng_to_h3(double lat, double lon, uint8_t resolution) {
    uint64_t out_h3 = 0;
    int32_t status = brhealth_coord_to_h3(lat, lon, resolution, &out_h3);
    if (status != 0) {
        throw EngineException("Coordenadas geodésicas ou resolução H3 inválidas");
    }
    return out_h3;
}

/**
 * @brief Classifica um código de diagnóstico CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008.
 *
 * @param cid10 Código diagnóstico da CID-10 (com ou sem ponto).
 * @return std::optional<uint8_t> Grupo CSAP (1 a 19) ou std::nullopt se não for CSAP.
 * @throws EngineException se o código CID for inválido ou malformatado.
 */
inline std::optional<uint8_t> classify_csap(std::string_view cid10) {
    std::string null_terminated(cid10);
    uint8_t group_id = 0;
    int32_t status = brhealth_classify_csap(null_terminated.c_str(), &group_id);
    if (status != 0) {
        throw EngineException("Erro ao classificar diagnóstico CID-10: " + null_terminated);
    }
    if (group_id == 0) {
        return std::nullopt;
    }
    return group_id;
}

/**
 * @brief Calcula o Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária.
 *
 * @param avoidable_cost Custo financeiro hospitalar direto evitável (R$).
 * @param investment Investimento orçamentário aplicado na Estratégia Saúde da Família (R$).
 * @param attributable_fraction Fração de impacto atribuível (0.0 a 1.0).
 * @return double ROI percentual.
 */
inline double compute_primary_care_roi(
    double avoidable_cost,
    double investment,
    double attributable_fraction
) {
    double out_roi = 0.0;
    int32_t status = brhealth_compute_primary_care_roi(
        avoidable_cost,
        investment,
        attributable_fraction,
        &out_roi
    );
    if (status != 0) {
        throw EngineException("Parâmetros inválidos para cálculo de ROI de Atenção Primária");
    }
    return out_roi;
}

} // namespace brhealth
