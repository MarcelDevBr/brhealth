// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

#pragma once

/**
 * @file brhealth.hpp
 * @brief Modern C++20 RAII Bindings for the BRHealth Columnar Analytics Engine.
 */

#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>

/// Estrutura canônica de Schema conforme a especificação Apache Arrow C Data Interface.
struct ArrowSchema {
    const char* format;
    const char* name;
    const char* metadata;
    int64_t flags;
    int64_t n_children;
    ArrowSchema** children;
    ArrowSchema* dictionary;
    void (*release)(ArrowSchema*);
    void* private_data;
};

/// Estrutura canônica de Array conforme a especificação Apache Arrow C Data Interface.
struct ArrowArray {
    int64_t length;
    int64_t null_count;
    int64_t offset;
    int64_t n_buffers;
    int64_t n_children;
    const void** buffers;
    ArrowArray** children;
    ArrowArray* dictionary;
    void (*release)(ArrowArray*);
    void* private_data;
};

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
    void* brhealth_client_create();
    void brhealth_client_destroy(void* handle);
    int32_t brhealth_fetch_mortality(
        void* handle,
        const char* uf,
        uint32_t year,
        void* out_array,
        void* out_schema
    );
    int32_t brhealth_fetch_source(
        void* handle,
        const char* source_id,
        const char* uf,
        uint32_t year,
        void* out_array,
        void* out_schema
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
 * @brief Gerenciador RAII de lote tabular `RecordBatch` Apache Arrow via C Data Interface.
 */
class ArrowRecordBatch {
private:
    ArrowArray array_{};
    ArrowSchema schema_{};
    bool valid_{false};

public:
    ArrowRecordBatch() = default;

    ~ArrowRecordBatch() {
        reset();
    }

    ArrowRecordBatch(const ArrowRecordBatch&) = delete;
    ArrowRecordBatch& operator=(const ArrowRecordBatch&) = delete;

    ArrowRecordBatch(ArrowRecordBatch&& other) noexcept {
        array_ = other.array_;
        schema_ = other.schema_;
        valid_ = other.valid_;
        other.array_ = {};
        other.schema_ = {};
        other.valid_ = false;
    }

    ArrowRecordBatch& operator=(ArrowRecordBatch&& other) noexcept {
        if (this != &other) {
            reset();
            array_ = other.array_;
            schema_ = other.schema_;
            valid_ = other.valid_;
            other.array_ = {};
            other.schema_ = {};
            other.valid_ = false;
        }
        return *this;
    }

    void reset() noexcept {
        if (valid_) {
            if (array_.release) {
                array_.release(&array_);
            }
            if (schema_.release) {
                schema_.release(&schema_);
            }
            valid_ = false;
        }
    }

    ArrowArray* array_ptr() noexcept { return &array_; }
    ArrowSchema* schema_ptr() noexcept { return &schema_; }
    void mark_valid() noexcept { valid_ = true; }
    [[nodiscard]] bool is_valid() const noexcept { return valid_; }
    [[nodiscard]] int64_t num_rows() const noexcept { return valid_ ? array_.length : 0; }
    [[nodiscard]] int64_t num_columns() const noexcept { return valid_ ? schema_.n_children : 0; }
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

/**
 * @brief Cliente RAII C++20 do motor analítico BRHealth.
 */
class Client {
private:
    void* handle_{nullptr};

public:
    Client() {
        handle_ = brhealth_client_create();
        if (!handle_) {
            throw EngineException("Falha ao instanciar o cliente nativo BRHealth");
        }
    }

    ~Client() {
        if (handle_) {
            brhealth_client_destroy(handle_);
            handle_ = nullptr;
        }
    }

    Client(const Client&) = delete;
    Client& operator=(const Client&) = delete;

    Client(Client&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }

    Client& operator=(Client&& other) noexcept {
        if (this != &other) {
            if (handle_) {
                brhealth_client_destroy(handle_);
            }
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }

    [[nodiscard]] static std::unique_ptr<Client> create() {
        return std::make_unique<Client>();
    }

    /**
     * @brief Ingesta e processa dados de mortalidade (DATASUS SIM).
     */
    ArrowRecordBatch fetch_mortality(std::string_view uf, uint32_t year) {
        return fetch_source("datasus_sim", uf, year);
    }

    /**
     * @brief Ingesta e processa qualquer fonte canônica registrada no motor.
     */
    ArrowRecordBatch fetch_source(std::string_view source_id, std::string_view uf, uint32_t year) {
        if (!handle_) {
            throw EngineException("Cliente BRHealth inválido ou já destruído");
        }
        std::string sid_str(source_id);
        std::string uf_str(uf);

        ArrowRecordBatch batch;
        int32_t status = brhealth_fetch_source(
            handle_,
            sid_str.c_str(),
            uf_str.c_str(),
            year,
            batch.array_ptr(),
            batch.schema_ptr()
        );
        if (status != 0) {
            throw EngineException("Falha ao consultar fonte analítica: " + sid_str);
        }
        batch.mark_valid();
        return batch;
    }
};

} // namespace brhealth
