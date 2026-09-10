// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Módulo de Bioestatística e Análise de Mortalidade Prematura.
//!
//! Implementa formulações matemáticas canônicas para avaliação epidemiológica de
//! mortalidade, notadamente os **Anos Potenciais de Vida Perdidos (APVP / YLL)** e a
//! **Padronização Direta de Taxas por Idade** aplicando a População Padrão Mundial da OMS.
//!
//! # Formulações Matemáticas
//!
//! ## 1. Anos Potenciais de Vida Perdidos (APVP / YLL)
//! Seja $n$ o número total de óbitos observados em uma população, $a_i$ a idade em anos
//! do indivíduo no momento do óbito e $L$ a idade limite de referência (canonicamente
//! $L = 70$ ou $L = 75$ anos segundo o Ministério da Saúde e a Organização Mundial da Saúde):
//!
//! $$\text{APVP} = \sum_{i=1}^{n} d_i \cdot (L - a_i), \quad \text{onde } d_i = \begin{cases} 1, & \text{se } a_i < L \\ 0, & \text{se } a_i \ge L \end{cases}$$
//!
//! A Taxa de APVP por $100.000$ habitantes na faixa etária menor que $L$ é definida por:
//!
//! $$\text{Taxa APVP} = \left( \frac{\text{APVP}}{\text{População}_{< L}} \right) \times 100.000$$
//!
//! ## 2. Padronização Direta de Taxas por Idade (OMS)
//! Para eliminar o efeito de estruturas etárias díspares entre diferentes jurisdições
//! ou momentos no tempo, calcula-se a taxa padronizada aplicando os pesos da População Padrão da OMS:
//!
//! $$\text{Taxa Padronizada Direta} = \sum_{k=1}^{M} w_k \cdot \left( \frac{O_k}{P_k} \right) \times 100.000$$
//!
//! Onde $w_k = \frac{P_k^{\text{padrão}}}{\sum P^{\text{padrão}}}$ representa o peso relativo da
//! faixa etária $k$, $O_k$ o número de óbitos observados e $P_k$ a população exposta local.

use arrow::array::{Array, AsArray};
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use serde::{Deserialize, Serialize};

use crate::domain::ports::outbound::PortError;

/// Idade limite padrão de referência do Ministério da Saúde do Brasil (70 anos).
pub const DEFAULT_CUTOFF_AGE_BR: u16 = 70;

/// Idade limite de referência recomendada pela OMS (75 anos).
pub const DEFAULT_CUTOFF_AGE_WHO: u16 = 75;

/// Pesos da População Padrão Mundial da OMS (2000–2025) por grupos etários quinquenais.
///
/// Total de indivíduos padrão = 100.000:
/// - 0-4 anos: 8.860
/// - 5-9 anos: 8.690
/// - 10-14 anos: 8.600
/// - 15-19 anos: 8.470
/// - 20-24 anos: 8.220
/// - 25-29 anos: 7.930
/// - 30-34 anos: 7.610
/// - 35-39 anos: 7.150
/// - 40-44 anos: 6.590
/// - 45-49 anos: 6.040
/// - 50-54 anos: 5.370
/// - 55-59 anos: 4.550
/// - 60-64 anos: 3.720
/// - 65-69 anos: 2.960
/// - 70-74 anos: 2.210
/// - 75-79 anos: 1.520
/// - 80-84 anos: 910
/// - 85+ anos: 600
pub const WHO_STANDARD_POPULATION_WEIGHTS: [f64; 18] = [
    0.0886, 0.0869, 0.0860, 0.0847, 0.0822, 0.0793, 0.0761, 0.0715, 0.0659, 0.0604, 0.0537, 0.0455,
    0.0372, 0.0296, 0.0221, 0.0152, 0.0091, 0.0060,
];

/// Resultado da consolidação de métricas de Anos Potenciais de Vida Perdidos (APVP).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApvpMetrics {
    /// Total absoluto de anos potenciais de vida perdidos ($\sum (L - a_i)$).
    pub total_apvp: u64,
    /// Total de óbitos prematuros ocorridos abaixo da idade limite $L$.
    pub premature_deaths: u64,
    /// Média de anos perdidos por óbito prematuro.
    pub mean_years_lost_per_death: f64,
    /// Idade limite de corte adotada ($L$).
    pub cutoff_age: u16,
    /// Taxa de APVP por 100.000 habitantes (se população fornecida).
    pub apvp_rate_per_100k: Option<f64>,
}

/// Calcula o total de Anos Potenciais de Vida Perdidos a partir de um slice de idades.
///
/// # Exemplos
///
/// ```
/// use brhealth_core::domain::analytics::mortality::compute_apvp;
///
/// let ages = [10, 25, 40, 72, 80]; // Óbitos aos 10, 25, 40, 72 e 80 anos
/// let apvp = compute_apvp(&ages, 70);
/// // (70 - 10) + (70 - 25) + (70 - 40) = 60 + 45 + 30 = 135
/// assert_eq!(apvp, 135);
/// ```
#[must_use]
pub fn compute_apvp(ages: &[u16], cutoff_age: u16) -> u64 {
    let mut total: u64 = 0;
    for &age in ages {
        if age < cutoff_age {
            total += u64::from(cutoff_age - age);
        }
    }
    total
}

/// Calcula a taxa de APVP por 100.000 habitantes.
///
/// # Exemplos
///
/// ```
/// use brhealth_core::domain::analytics::mortality::compute_apvp_rate;
///
/// let rate = compute_apvp_rate(1350, 50_000).unwrap();
/// assert_eq!(rate, 2700.0);
/// ```
pub fn compute_apvp_rate(apvp: u64, pop_under_cutoff: u64) -> Result<f64, PortError> {
    if pop_under_cutoff == 0 {
        return Err(PortError::ValidationError(
            "População de referência para cálculo da taxa de APVP não pode ser zero".into(),
        ));
    }
    #[allow(clippy::cast_precision_loss)]
    let rate = (apvp as f64 / pop_under_cutoff as f64) * 100_000.0;
    Ok(rate)
}

/// Avalia vetorizadamente um `RecordBatch` Arrow e calcula métricas completas de APVP.
///
/// A coluna de idade pode ser `UInt16`, `Int32` ou `UInt8`.
pub fn compute_batch_apvp(
    batch: &RecordBatch,
    age_column_name: &str,
    cutoff_age: u16,
    reference_population_under_cutoff: Option<u64>,
) -> Result<ApvpMetrics, PortError> {
    let col = batch.column_by_name(age_column_name).ok_or_else(|| {
        PortError::ValidationError(format!(
            "Coluna de idade '{age_column_name}' não encontrada no lote"
        ))
    })?;

    let mut total_apvp: u64 = 0;
    let mut premature_deaths: u64 = 0;

    match col.data_type() {
        DataType::UInt16 => {
            let arr = col.as_primitive::<arrow::datatypes::UInt16Type>();
            for i in 0..arr.len() {
                if arr.is_valid(i) {
                    let age = arr.value(i);
                    if age < cutoff_age {
                        total_apvp += u64::from(cutoff_age - age);
                        premature_deaths += 1;
                    }
                }
            }
        }
        DataType::Int32 => {
            let arr = col.as_primitive::<arrow::datatypes::Int32Type>();
            for i in 0..arr.len() {
                if arr.is_valid(i) {
                    let val = arr.value(i);
                    if val >= 0 {
                        let age = u16::try_from(val).unwrap_or(u16::MAX);
                        if age < cutoff_age {
                            total_apvp += u64::from(cutoff_age - age);
                            premature_deaths += 1;
                        }
                    }
                }
            }
        }
        DataType::UInt8 => {
            let arr = col.as_primitive::<arrow::datatypes::UInt8Type>();
            for i in 0..arr.len() {
                if arr.is_valid(i) {
                    let age = u16::from(arr.value(i));
                    if age < cutoff_age {
                        total_apvp += u64::from(cutoff_age - age);
                        premature_deaths += 1;
                    }
                }
            }
        }
        other => {
            return Err(PortError::ValidationError(format!(
                "Tipo de dado não suportado para coluna de idade: {other:?}"
            )));
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let mean_years_lost_per_death = if premature_deaths > 0 {
        total_apvp as f64 / premature_deaths as f64
    } else {
        0.0
    };

    let apvp_rate_per_100k = match reference_population_under_cutoff {
        Some(pop) => Some(compute_apvp_rate(total_apvp, pop)?),
        None => None,
    };

    Ok(ApvpMetrics {
        total_apvp,
        premature_deaths,
        mean_years_lost_per_death,
        cutoff_age,
        apvp_rate_per_100k,
    })
}

/// Calcula a Taxa Padronizada Direta de Mortalidade por 100.000 habitantes
/// utilizando os pesos populacionais quinquenais da OMS (2000–2025).
///
/// Requer dois slices com exatamente 18 elementos correspondentes às 18 faixas etárias padrão:
/// `[0-4, 5-9, ..., 80-84, 85+]`.
pub fn compute_age_standardized_mortality_rate(
    observed_deaths_by_age: &[u64],
    local_pop_by_age: &[u64],
) -> Result<f64, PortError> {
    if observed_deaths_by_age.len() != 18 || local_pop_by_age.len() != 18 {
        return Err(PortError::ValidationError(
            "Padronização Direta da OMS requer exatamente 18 faixas etárias quinquenais (0 a 85+ anos)".into(),
        ));
    }

    let mut standardized_rate_sum = 0.0;

    for i in 0..18 {
        let pop = local_pop_by_age[i];
        if pop > 0 {
            #[allow(clippy::cast_precision_loss)]
            let specific_rate = (observed_deaths_by_age[i] as f64) / (pop as f64);
            let weight = WHO_STANDARD_POPULATION_WEIGHTS[i];
            standardized_rate_sum += weight * specific_rate;
        }
    }

    Ok(standardized_rate_sum * 100_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::UInt16Array;
    use arrow::datatypes::{Field, Schema};
    use std::sync::Arc;

    #[test]
    fn test_compute_apvp_basic() {
        let ages = [0, 20, 50, 70, 85];
        let apvp_70 = compute_apvp(&ages, 70);
        // (70 - 0) + (70 - 20) + (70 - 50) = 70 + 50 + 20 = 140
        assert_eq!(apvp_70, 140);

        let apvp_75 = compute_apvp(&ages, 75);
        // (75-0) + (75-20) + (75-50) + (75-70) = 75 + 55 + 25 + 5 = 160
        assert_eq!(apvp_75, 160);
    }

    #[test]
    fn test_compute_apvp_rate() {
        let rate = compute_apvp_rate(2500, 100_000).unwrap();
        assert!((rate - 2500.0).abs() < 1e-6);

        let err = compute_apvp_rate(100, 0);
        assert!(err.is_err());
    }

    #[test]
    fn test_compute_batch_apvp_arrow() {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "age_years",
            DataType::UInt16,
            false,
        )]));
        let array = Arc::new(UInt16Array::from(vec![10, 30, 60, 75, 80]));
        let batch = RecordBatch::try_new(schema, vec![array]).unwrap();

        let metrics = compute_batch_apvp(&batch, "age_years", 70, Some(50_000)).unwrap();
        assert_eq!(metrics.total_apvp, (70 - 10) + (70 - 30) + (70 - 60)); // 60 + 40 + 10 = 110
        assert_eq!(metrics.premature_deaths, 3);
        assert!((metrics.mean_years_lost_per_death - (110.0 / 3.0)).abs() < 1e-4);
        assert!(metrics.apvp_rate_per_100k.is_some());
    }

    #[test]
    fn test_compute_age_standardized_mortality_rate() {
        let deaths = [10u64; 18];
        let pop = [10_000u64; 18];

        let rate = compute_age_standardized_mortality_rate(&deaths, &pop).unwrap();
        // Se a taxa em todas as faixas é 10 / 10000 = 0.001 (100 por 100k), a taxa padronizada deve ser exatamente 100 por 100k
        assert!((rate - 100.0).abs() < 1.0);
    }
}
