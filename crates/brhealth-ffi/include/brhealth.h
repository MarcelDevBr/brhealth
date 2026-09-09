/**
 * Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
 * Licensed under the GNU Affero General Public License v3 (AGPLv3)
 * or a commercial license agreement directly with the author.
 *
 * BRHealth C-ABI and Arrow C Data Interface Header
 */

#ifndef BRHEALTH_H
#define BRHEALTH_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Códigos de status retornados pelas funções BRHealth C-ABI */
#define BRHEALTH_SUCCESS 0
#define BRHEALTH_ERR_NULL_PTR -1
#define BRHEALTH_ERR_INVALID_ARG -2
#define BRHEALTH_ERR_TRANSFORM_FAILED -3

/* 
 * Estrutura Arrow C Data Interface: ArrowSchema
 * Conforme especificação oficial Apache Arrow.
 */
struct ArrowSchema {
    const char* format;
    const char* name;
    const char* metadata;
    int64_t flags;
    int64_t n_children;
    struct ArrowSchema** children;
    struct ArrowSchema* dictionary;
    void (*release)(struct ArrowSchema*);
    void* private_data;
};

/* 
 * Estrutura Arrow C Data Interface: ArrowArray
 * Conforme especificação oficial Apache Arrow.
 */
struct ArrowArray {
    int64_t length;
    int64_t null_count;
    int64_t offset;
    int64_t n_buffers;
    int64_t n_children;
    const void** buffers;
    struct ArrowArray** children;
    struct ArrowArray* dictionary;
    void (*release)(struct ArrowArray*);
    void* private_data;
};

/**
 * Retorna a versão canônica da biblioteca BRHealth em string terminada em null.
 */
const char* brhealth_version(void);

/**
 * Calcula o Dígito Verificador (DV) do IBGE (Luhn Módulo 10) para um código municipal de 6 dígitos.
 * Grava o dígito verificador (0 a 9) em out_dv.
 */
int32_t brhealth_calculate_ibge_dv(const char* ibge_6digits, uint8_t* out_dv);

/**
 * Converte latitude e longitude em um índice hexagonal discreto Uber H3 de 64 bits.
 */
int32_t brhealth_coord_to_h3(double lat, double lon, uint8_t resolution, uint64_t* out_h3);

/**
 * Classifica um código CID-10 conforme os 19 grupos de CSAP (Portaria MS/SAS nº 221/2008).
 * Grava o grupo (1 a 19) em out_group_id, ou 0 caso não pertença às CSAP.
 */
int32_t brhealth_classify_csap(const char* cid10, uint8_t* out_group_id);

/**
 * Calcula o Retorno sobre o Investimento (ROI) em Saúde Coletiva na Atenção Primária à Saúde.
 */
int32_t brhealth_compute_primary_care_roi(
    double avoidable_cost,
    double investment,
    double attributable_fraction,
    double* out_roi
);

/**
 * Exporta um lote RecordBatch Apache Arrow para estruturas da Arrow C Data Interface (Zero-Copy).
 */
int32_t brhealth_export_arrow_batch(
    const void* batch_ptr,
    struct ArrowArray* out_array,
    struct ArrowSchema* out_schema
);

#ifdef __cplusplus
}
#endif

#endif /* BRHEALTH_H */
