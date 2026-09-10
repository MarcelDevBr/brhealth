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

    let bytes = trimmed.as_bytes();
    let mut sum: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if !b.is_ascii_digit() {
            return Err(PortError::ValidationError(format!(
                "Caractere inválido no código IBGE: '{}'",
                b as char
            )));
        }
        let digit = (b - b'0') as u32;
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
            let b7 = trimmed.as_bytes()[6];
            if !b7.is_ascii_digit() {
                return Err(PortError::ValidationError(
                    "7º dígito do código IBGE não é numérico".into(),
                ));
            }
            let current_dv = b7 - b'0';

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

/// Registro de transição territorial histórica de municípios brasileiros (1970–2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalTransition {
    /// Código legado de 6 dígitos
    pub legacy_code_6: &'static str,
    /// Código legado com DV de 7 dígitos
    pub legacy_code_7: &'static str,
    /// Código canônico atual do IBGE de 7 dígitos
    pub canonical_code_7: &'static str,
    /// Ano da transição territorial (CF/88, emancipação, etc.)
    pub transition_year: u16,
    /// Descrição formal da alteração territorial
    pub description: &'static str,
}

/// Tabela estática de mapeamento de transições territoriais históricas.
pub const HISTORICAL_TRANSITIONS: &[HistoricalTransition] = &[
    HistoricalTransition {
        legacy_code_6: "200001",
        legacy_code_7: "2000013",
        canonical_code_7: "2605459", // Fernando de Noronha (PE)
        transition_year: 1988,
        description: "Território Federal de Fernando de Noronha incorporado ao estado de Pernambuco",
    },
    HistoricalTransition {
        legacy_code_6: "260545",
        legacy_code_7: "2605459",
        canonical_code_7: "2605459",
        transition_year: 1988,
        description: "Fernando de Noronha canônico (PE)",
    },
    // Transições decorrentes da criação do estado do Tocantins (desmembrado de Goiás em 1988)
    HistoricalTransition {
        legacy_code_6: "520210",
        legacy_code_7: "5202101",
        canonical_code_7: "1702107", // Araguaína
        transition_year: 1988,
        description: "Araguaína transferido de Goiás para Tocantins",
    },
    HistoricalTransition {
        legacy_code_6: "520930",
        legacy_code_7: "5209308",
        canonical_code_7: "1709300", // Gurupi
        transition_year: 1988,
        description: "Gurupi transferido de Goiás para Tocantins",
    },
    HistoricalTransition {
        legacy_code_6: "521360",
        legacy_code_7: "5213603",
        canonical_code_7: "1713205", // Miracema do Tocantins
        transition_year: 1988,
        description: "Miracema do Norte (GO) renomeado e transferido para Miracema do Tocantins",
    },
    HistoricalTransition {
        legacy_code_6: "521780",
        legacy_code_7: "5217800",
        canonical_code_7: "1718204", // Porto Nacional
        transition_year: 1988,
        description: "Porto Nacional transferido de Goiás para Tocantins",
    },
    HistoricalTransition {
        legacy_code_6: "521660",
        legacy_code_7: "5216604",
        canonical_code_7: "1716109", // Paraíso do Tocantins
        transition_year: 1988,
        description: "Paraíso do Norte de Goiás transferido para Paraíso do Tocantins",
    },
];

/// Reconcilia códigos municipais históricos com a malha canônica do IBGE de 2026.
///
/// Caso o código pertença a uma transição territorial histórica (ex: municípios do antigo norte
/// de Goiás transferidos para o Tocantins em 1988 ou o antigo Território de Fernando de Noronha),
/// o código canônico contemporâneo é retornado. Caso contrário, é aplicada a harmonização padrão.
///
/// # Exemplos
///
/// ```rust
/// use brhealth_core::domain::transforms::ibge::reconcile_historical_ibge_code;
///
/// // Antigo código de Fernando de Noronha
/// assert_eq!(reconcile_historical_ibge_code("200001", Some(1980)).unwrap(), "2605459");
///
/// // Araguaína com código histórico de Goiás antes de 1988
/// assert_eq!(reconcile_historical_ibge_code("520210", Some(1985)).unwrap(), "1702107");
///
/// // Município sem alteração territorial segue o cálculo padrão
/// assert_eq!(reconcile_historical_ibge_code("355030", None).unwrap(), "3550308");
/// ```
pub fn reconcile_historical_ibge_code(
    raw_code: &str,
    reference_year: Option<u16>,
) -> Result<String, PortError> {
    let trimmed = raw_code.trim();
    for transition in HISTORICAL_TRANSITIONS {
        if trimmed == transition.legacy_code_6 || trimmed == transition.legacy_code_7 {
            if let Some(year) = reference_year {
                if year <= transition.transition_year {
                    return Ok(transition.canonical_code_7.to_string());
                }
            } else {
                return Ok(transition.canonical_code_7.to_string());
            }
        }
    }

    harmonize_ibge_code(raw_code)
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

    #[test]
    fn test_historical_transitions() {
        // Fernando de Noronha histórico -> canônico
        assert_eq!(
            reconcile_historical_ibge_code("200001", Some(1980)).unwrap(),
            "2605459"
        );
        assert_eq!(
            reconcile_historical_ibge_code("2000013", Some(1985)).unwrap(),
            "2605459"
        );

        // Tocantins (Araguaína antigo de GO -> TO)
        assert_eq!(
            reconcile_historical_ibge_code("520210", Some(1987)).unwrap(),
            "1702107"
        );
        assert_eq!(
            reconcile_historical_ibge_code("520930", Some(1982)).unwrap(),
            "1709300"
        );

        // Município regular sem transição segue padrão
        assert_eq!(
            reconcile_historical_ibge_code("355030", None).unwrap(),
            "3550308"
        );
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
