// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! # Harmonizador Territorial do IBGE
//!
//! Este módulo implementa a validação matemática e a harmonização dos códigos de municípios
//! brasileiros segundo a especificação técnica oficial do IBGE.
//!
//! ## Formulação Matemática (Módulo 10 - Variante de Luhn)
//!
//! Dado o vetor de dígitos do código municipal de 6 dígitos:
//! $$D = [d_1, d_2, d_3, d_4, d_5, d_6] \in \{0, \dots, 9\}^6$$
//!
//! E o vetor de pesos oficiais alternados:
//! $$W = [1, 2, 1, 2, 1, 2]$$
//!
//! Para cada posição $i \in \{1, \dots, 6\}$:
//! $$p_i = d_i \times w_i$$
//! $$s_i = \lfloor p_i / 10 \rfloor + (p_i \pmod{10})$$
//!
//! A soma ponderada total é dada por:
//! $$S = \sum_{i=1}^{6} s_i$$
//!
//! O resto $R$ da divisão euclidiana por 10:
//! $$R = S \pmod{10}$$
//!
//! O Dígito Verificador (DV) é definido por:
//! $$\text{DV} = (10 - R) \pmod{10}$$
//! *(onde $\text{DV} = 0$ caso $R = 0$)*.

use super::super::ports::outbound::PortError;

/// Pesos oficiais alternados para o cálculo do Dígito Verificador do IBGE (Módulo 10)
const IBGE_WEIGHTS: [u32; 6] = [1, 2, 1, 2, 1, 2];

/// Calcula o 7º dígito verificador para um código de município do IBGE de 6 dígitos.
///
/// # Exemplos
///
/// ```rust
/// use brhealth_core::calculate_ibge_dv;
///
/// // São Paulo / SP: 355030 -> DV 8
/// let dv_sp = calculate_ibge_dv("355030").unwrap();
/// assert_eq!(dv_sp, 8);
///
/// // Belo Horizonte / MG: 310620 -> DV 0
/// let dv_bh = calculate_ibge_dv("310620").unwrap();
/// assert_eq!(dv_bh, 0);
/// ```
///
/// # Erros
///
/// Retorna `PortError::ValidationError` se a string não contiver exatamente 6 dígitos numéricos.
pub fn calculate_ibge_dv(code_6_digits: &str) -> Result<u8, PortError> {
    let trimmed = code_6_digits.trim();
    if trimmed.len() != 6 {
        return Err(PortError::ValidationError(format!(
            "Código IBGE deve conter exatamente 6 dígitos para cálculo de DV, recebido: '{}'",
            code_6_digits
        )));
    }

    let mut sum: u32 = 0;
    for (i, ch) in trimmed.chars().enumerate() {
        let digit = ch.to_digit(10).ok_or_else(|| {
            PortError::ValidationError(format!("Caractere inválido no código IBGE: '{}'", ch))
        })?;

        let product = digit * IBGE_WEIGHTS[i];
        let term = (product / 10) + (product % 10);
        sum += term;
    }

    let remainder = sum % 10;
    let dv = if remainder == 0 {
        0
    } else {
        (10 - remainder) as u8
    };

    Ok(dv)
}

/// Harmoniza um código municipal (de 6 ou 7 dígitos) convertendo-o para a representação canônica de 7 dígitos.
///
/// Caso o código já possua 7 dígitos, seu dígito verificador é rigorosamente validado.
///
/// # Exemplos
///
/// ```rust
/// use brhealth_core::harmonize_ibge_code;
///
/// // Completa código de 6 dígitos adicionando o DV oficial
/// assert_eq!(harmonize_ibge_code("355030").unwrap(), "3550308");
///
/// // Valida e preserva código canônico de 7 dígitos
/// assert_eq!(harmonize_ibge_code("3550308").unwrap(), "3550308");
///
/// // Erro caso o DV existente seja inconsistente com a fórmula oficial
/// assert!(harmonize_ibge_code("3550309").is_err());
/// ```
pub fn harmonize_ibge_code(raw_code: &str) -> Result<String, PortError> {
    let trimmed = raw_code.trim();
    match trimmed.len() {
        7 => {
            let base_6 = &trimmed[0..6];
            let expected_dv = calculate_ibge_dv(base_6)?;
            let current_dv = trimmed
                .chars()
                .nth(6)
                .ok_or_else(|| PortError::ValidationError("7º dígito ausente".into()))?
                .to_digit(10)
                .ok_or_else(|| {
                    PortError::ValidationError("7º dígito do código IBGE não é numérico".into())
                })? as u8;

            if current_dv != expected_dv {
                return Err(PortError::ValidationError(format!(
                    "Código IBGE '{}' possui DV inválido. Esperado: {}, encontrado: {}",
                    trimmed, expected_dv, current_dv
                )));
            }
            Ok(trimmed.to_string())
        }
        6 => {
            let dv = calculate_ibge_dv(trimmed)?;
            Ok(format!("{}{}", trimmed, dv))
        }
        other => Err(PortError::ValidationError(format!(
            "Código IBGE possui tamanho inválido ({}), esperado 6 ou 7 dígitos",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_sao_paulo_dv() {
        let dv = calculate_ibge_dv("355030").unwrap();
        assert_eq!(dv, 8);
        assert_eq!(harmonize_ibge_code("355030").unwrap(), "3550308");
        assert_eq!(harmonize_ibge_code("3550308").unwrap(), "3550308");
    }

    #[test]
    fn test_rio_de_janeiro_dv() {
        let dv = calculate_ibge_dv("330455").unwrap();
        assert_eq!(dv, 7);
        assert_eq!(harmonize_ibge_code("330455").unwrap(), "3304557");
        assert_eq!(harmonize_ibge_code("3304557").unwrap(), "3304557");
    }

    #[test]
    fn test_belo_horizonte_dv() {
        let dv = calculate_ibge_dv("310620").unwrap();
        assert_eq!(dv, 0);
        assert_eq!(harmonize_ibge_code("310620").unwrap(), "3106200");
        assert_eq!(harmonize_ibge_code("3106200").unwrap(), "3106200");
    }

    #[test]
    fn test_campinas_dv() {
        let dv = calculate_ibge_dv("350950").unwrap();
        assert_eq!(dv, 2);
        assert_eq!(harmonize_ibge_code("350950").unwrap(), "3509502");
        assert_eq!(harmonize_ibge_code("3509502").unwrap(), "3509502");
    }

    #[test]
    fn test_salvador_dv() {
        let dv = calculate_ibge_dv("292740").unwrap();
        assert_eq!(dv, 8);
        assert_eq!(harmonize_ibge_code("292740").unwrap(), "2927408");
        assert_eq!(harmonize_ibge_code("2927408").unwrap(), "2927408");
    }

    #[test]
    fn test_invalid_length() {
        assert!(calculate_ibge_dv("12345").is_err());
        assert!(calculate_ibge_dv("1234567").is_err());
        assert!(harmonize_ibge_code("1234").is_err());
        assert!(harmonize_ibge_code("12345678").is_err());
    }

    #[test]
    fn test_invalid_chars() {
        assert!(calculate_ibge_dv("35503A").is_err());
        assert!(harmonize_ibge_code("35503A").is_err());
        assert!(harmonize_ibge_code("355030A").is_err());
    }

    #[test]
    fn test_invalid_existing_dv() {
        assert!(harmonize_ibge_code("3550309").is_err());
    }

    // Property-Based Testing com proptest
    proptest! {
        #[test]
        fn prop_test_ibge_dv_invariants(code in "[0-9]{6}") {
            let dv_res = calculate_ibge_dv(&code);
            prop_assert!(dv_res.is_ok());
            let dv = dv_res.unwrap();
            // Invariante 1: O DV deve ser sempre um único dígito decimal [0, 9]
            prop_assert!(dv <= 9);

            // Invariante 2: A harmonização do código deve ser idempotente
            let harm_res = harmonize_ibge_code(&code);
            prop_assert!(harm_res.is_ok());
            let harm_7 = harm_res.unwrap();
            prop_assert_eq!(harm_7.len(), 7);

            // Validar que o código gerado é aceito por harmonize_ibge_code
            let re_harm = harmonize_ibge_code(&harm_7);
            prop_assert!(re_harm.is_ok());
            prop_assert_eq!(re_harm.unwrap(), harm_7);
        }

        #[test]
        fn prop_test_rejects_arbitrary_short_strings(s in "[0-9]{0,5}") {
            prop_assert!(calculate_ibge_dv(&s).is_err());
            prop_assert!(harmonize_ibge_code(&s).is_err());
        }

        #[test]
        fn prop_test_rejects_arbitrary_long_strings(s in "[0-9]{8,20}") {
            prop_assert!(calculate_ibge_dv(&s).is_err());
            prop_assert!(harmonize_ibge_code(&s).is_err());
        }
    }
}
