// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Harmonizador de Vocabulários Farmacêuticos (ATC e RxNorm).
//!
//! Padroniza princípios ativos e classes terapêuticas para monitoramento de compras
//! públicas em saúde (BPS/CMED/Anvisa), dispensação na Atenção Básica (Farmácia Popular)
//! e interoperabilidade internacional com o vocabulário **RxNorm** da National Library
//! of Medicine (NLM) e a classificação **ATC** (*Anatomical Therapeutic Chemical*) da OMS.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Metadados de um fármaco essencial padronizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardDrugConcept {
    /// Código ATC da Organização Mundial da Saúde (ex: `"A10BA02"`).
    pub atc_code: &'static str,
    /// Nome do princípio ativo ou substância (Denominação Comum Brasileira - DCB).
    pub active_ingredient: &'static str,
    /// Identificador de conceito no RxNorm (RxCUI).
    pub rxnorm_cui: &'static str,
    /// Grupo anatômico principal segundo a classificação ATC.
    pub anatomical_group: &'static str,
}

static PHARMACY_REGISTRY: OnceLock<HashMap<&'static str, StandardDrugConcept>> = OnceLock::new();

fn init_pharmacy_registry() -> HashMap<&'static str, StandardDrugConcept> {
    HashMap::from([
        // Antidiabéticos (A10)
        (
            "A10BA02",
            StandardDrugConcept {
                atc_code: "A10BA02",
                active_ingredient: "Metformina",
                rxnorm_cui: "6809",
                anatomical_group: "Trato Digestivo e Metabolismo",
            },
        ),
        (
            "A10BB01",
            StandardDrugConcept {
                atc_code: "A10BB01",
                active_ingredient: "Glibenclamida",
                rxnorm_cui: "4815",
                anatomical_group: "Trato Digestivo e Metabolismo",
            },
        ),
        (
            "A10AC01",
            StandardDrugConcept {
                atc_code: "A10AC01",
                active_ingredient: "Insulina NPH",
                rxnorm_cui: "5856",
                anatomical_group: "Trato Digestivo e Metabolismo",
            },
        ),
        // Anti-hipertensivos e Cardiovasculares (C02, C03, C07, C09)
        (
            "C09CA01",
            StandardDrugConcept {
                atc_code: "C09CA01",
                active_ingredient: "Losartana Potássica",
                rxnorm_cui: "52175",
                anatomical_group: "Aparelho Cardiovascular",
            },
        ),
        (
            "C09AA02",
            StandardDrugConcept {
                atc_code: "C09AA02",
                active_ingredient: "Maleato de Enalapril",
                rxnorm_cui: "3827",
                anatomical_group: "Aparelho Cardiovascular",
            },
        ),
        (
            "C03AA03",
            StandardDrugConcept {
                atc_code: "C03AA03",
                active_ingredient: "Hidroclorotiazida",
                rxnorm_cui: "5487",
                anatomical_group: "Aparelho Cardiovascular",
            },
        ),
        (
            "C07AB03",
            StandardDrugConcept {
                atc_code: "C07AB03",
                active_ingredient: "Atenolol",
                rxnorm_cui: "1202",
                anatomical_group: "Aparelho Cardiovascular",
            },
        ),
        (
            "B01AC06",
            StandardDrugConcept {
                atc_code: "B01AC06",
                active_ingredient: "Ácido Acetilsalicílico",
                rxnorm_cui: "1191",
                anatomical_group: "Sangue e Órgãos Hematopoéticos",
            },
        ),
        // Respiratórios (R03)
        (
            "R03AC02",
            StandardDrugConcept {
                atc_code: "R03AC02",
                active_ingredient: "Sulfato de Salbutamol",
                rxnorm_cui: "435",
                anatomical_group: "Aparelho Respiratório",
            },
        ),
        (
            "R03BA01",
            StandardDrugConcept {
                atc_code: "R03BA01",
                active_ingredient: "Dipropionato de Beclometasona",
                rxnorm_cui: "1343",
                anatomical_group: "Aparelho Respiratório",
            },
        ),
        // Antimicrobianos (J01)
        (
            "J01CA04",
            StandardDrugConcept {
                atc_code: "J01CA04",
                active_ingredient: "Amoxicilina",
                rxnorm_cui: "723",
                anatomical_group: "Anti-infecciosos Gerais para Uso Sistêmico",
            },
        ),
        (
            "J01FA10",
            StandardDrugConcept {
                atc_code: "J01FA10",
                active_ingredient: "Azitromicina",
                rxnorm_cui: "18631",
                anatomical_group: "Anti-infecciosos Gerais para Uso Sistêmico",
            },
        ),
    ])
}

/// Harmonizador e pesquisador de ontologias farmacêuticas.
#[derive(Debug, Default, Clone)]
pub struct PharmacyHarmonizer;

impl PharmacyHarmonizer {
    /// Cria uma nova instância do harmonizador farmacêutico.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Obtém os metadados padronizados a partir de um código ATC da OMS.
    #[must_use]
    pub fn lookup_atc(&self, atc_code: &str) -> Option<&'static StandardDrugConcept> {
        let cleaned = atc_code.trim().to_uppercase();
        PHARMACY_REGISTRY
            .get_or_init(init_pharmacy_registry)
            .get(cleaned.as_str())
    }

    /// Mapeia um código ATC para o identificador conceitual RxNorm (RxCUI).
    #[must_use]
    pub fn map_atc_to_rxnorm(&self, atc_code: &str) -> Option<&'static str> {
        self.lookup_atc(atc_code).map(|d| d.rxnorm_cui)
    }

    /// Retorna a denominação do grupo anatômico principal da classificação ATC (1º nível).
    #[must_use]
    pub fn get_anatomical_level1(&self, atc_code: &str) -> Option<&'static str> {
        let cleaned = atc_code.trim().to_uppercase();
        let letter = cleaned.chars().next()?;
        match letter {
            'A' => Some("Trato Digestivo e Metabolismo"),
            'B' => Some("Sangue e Órgãos Hematopoéticos"),
            'C' => Some("Aparelho Cardiovascular"),
            'D' => Some("Medicamentos Dermatológicos"),
            'G' => Some("Aparelho Geniturinário e Hormônios Sexuais"),
            'H' => Some("Preparações Hormonais Sistêmicas"),
            'J' => Some("Anti-infecciosos Gerais para Uso Sistêmico"),
            'L' => Some("Agentes Antineoplásicos e Imunomoduladores"),
            'M' => Some("Aparelho Locomotor"),
            'N' => Some("Sistema Nervoso"),
            'P' => Some("Produtos Antiparasitários"),
            'R' => Some("Aparelho Respiratório"),
            'S' => Some("Órgãos dos Sentidos"),
            'V' => Some("Vários"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atc_lookup_metformin() {
        let harmonizer = PharmacyHarmonizer::new();
        let concept = harmonizer.lookup_atc("A10BA02").unwrap();
        assert_eq!(concept.active_ingredient, "Metformina");
        assert_eq!(concept.rxnorm_cui, "6809");
        assert_eq!(
            harmonizer.map_atc_to_rxnorm("A10BA02"),
            Some("6809")
        );
    }

    #[test]
    fn test_anatomical_group() {
        let harmonizer = PharmacyHarmonizer::new();
        assert_eq!(
            harmonizer.get_anatomical_level1("C09CA01"),
            Some("Aparelho Cardiovascular")
        );
        assert_eq!(
            harmonizer.get_anatomical_level1("R03AC02"),
            Some("Aparelho Respiratório")
        );
    }
}
