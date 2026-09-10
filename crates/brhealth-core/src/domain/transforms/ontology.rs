// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Harmonizador Universal de Ontologias Médicas (CID-10, CID-11 e SNOMED-CT).
//!
//! Implementa validação estrutural de consistência biológica (incompatibilidades
//! estritas de sexo e idade) e tabelas de equivalência transversal recomendadas
//! pela Organização Mundial da Saúde (OMS) para estudos epidemiológicos longitudinais.
//!
//! # Formulação Matemática da Consistência Biológica
//!
//! Seja um evento clínico ou vital $E = (\text{CID}, s, a)$, onde $\text{CID} \in \mathcal{C}$,
//! $s \in \{\text{'M'}, \text{'F'}, \text{'U'}\}$ representa o sexo biológico e $a \in \mathbb{N}_0$
//! a idade em anos:
//!
//! $$\text{Validade}(E) = \begin{cases}
//! \text{False}, & \text{se } \text{CID} \in \mathcal{C}_{\text{feminino}} \land s = \text{'M'} \\
//! \text{False}, & \text{se } \text{CID} \in \mathcal{C}_{\text{masculino}} \land s = \text{'F'} \\
//! \text{False}, & \text{se } \text{CID} \in \mathcal{C}_{\text{neonatal}} \land a > 1 \\
//! \text{False}, & \text{se } \text{CID} \in \mathcal{C}_{\text{senil}} \land a < 15 \\
//! \text{True}, & \text{caso contrário}
//! \end{cases}$$

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::domain::analytics::csap::{CsapGroup, classify_cid10};
use crate::domain::ports::outbound::PortError;

/// Os 22 Capítulos canônicos da CID-10 definidos pela Organização Mundial da Saúde (OMS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Icd10Chapter {
    /// I. Algumas doenças infecciosas e parasitárias (A00-B99)
    InfectiousAndParasitic = 1,
    /// II. Neoplasias [tumores] (C00-D48)
    Neoplasms = 2,
    /// III. Doenças do sangue e dos órgãos hematopoéticos e alguns transtornos imunitários (D50-D89)
    BloodAndImmune = 3,
    /// IV. Doenças endócrinas, nutricionais e metabólicas (E00-E90)
    EndocrineNutritionalMetabolic = 4,
    /// V. Transtornos mentais e comportamentais (F00-F99)
    MentalAndBehavioural = 5,
    /// VI. Doenças do sistema nervoso (G00-G99)
    NervousSystem = 6,
    /// VII. Doenças do olho e anexos (H00-H59)
    EyeAndAdnexa = 7,
    /// VIII. Doenças do ouvido e da apófise mastoide (H60-H95)
    EarAndMastoid = 8,
    /// IX. Doenças do aparelho circulatório (I00-I99)
    CirculatorySystem = 9,
    /// X. Doenças do aparelho respiratório (J00-J99)
    RespiratorySystem = 10,
    /// XI. Doenças do aparelho digestivo (K00-K93)
    DigestiveSystem = 11,
    /// XII. Doenças da pele e do tecido subcutâneo (L00-L99)
    SkinAndSubcutaneous = 12,
    /// XIII. Doenças do sistema osteomuscular e do tecido conjuntivo (M00-M99)
    MusculoskeletalAndConnective = 13,
    /// XIV. Doenças do aparelho geniturinário (N00-N99)
    GenitourinarySystem = 14,
    /// XV. Gravidez, parto e puerpério (O00-O99)
    PregnancyChildbirthPuerperium = 15,
    /// XVI. Algumas afecções originadas no período perinatal (P00-P96)
    PerinatalPeriod = 16,
    /// XVII. Malformações congênitas, deformidades e anomalias cromossômicas (Q00-Q99)
    CongenitalMalformations = 17,
    /// XVIII. Sintomas, sinais e achados anormais de exames clínicos e de laboratório (R00-R99)
    SymptomsAndAbnormalFindings = 18,
    /// XIX. Lesões, envenenamento e algumas outras consequências de causas externas (S00-T98)
    InjuryPoisoningExternalCauses = 19,
    /// XX. Causas externas de morbidade e de mortalidade (V01-Y98)
    ExternalCausesMorbidityMortality = 20,
    /// XXI. Fatores que influenciam o estado de saúde e o contato com os serviços de saúde (Z00-Z99)
    HealthStatusFactors = 21,
    /// XXII. Códigos para propósitos especiais (U00-U85)
    SpecialPurposes = 22,
}

impl Icd10Chapter {
    /// Determina o Capítulo da CID-10 a partir de um código alfanumérico com $O(1)$ e zero-alocação.
    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        let b = trimmed.as_bytes();
        if b.len() < 3 {
            return None;
        }

        let letter = b[0].to_ascii_uppercase();
        let d1 = (b[1] as char).to_digit(10)? as u8;
        let d2 = (b[2] as char).to_digit(10)? as u8;
        let num = d1 * 10 + d2;

        match letter {
            b'A' | b'B' => Some(Self::InfectiousAndParasitic),
            b'C' => Some(Self::Neoplasms),
            b'D' => {
                if num <= 48 {
                    Some(Self::Neoplasms)
                } else {
                    Some(Self::BloodAndImmune)
                }
            }
            b'E' => Some(Self::EndocrineNutritionalMetabolic),
            b'F' => Some(Self::MentalAndBehavioural),
            b'G' => Some(Self::NervousSystem),
            b'H' => {
                if num <= 59 {
                    Some(Self::EyeAndAdnexa)
                } else {
                    Some(Self::EarAndMastoid)
                }
            }
            b'I' => Some(Self::CirculatorySystem),
            b'J' => Some(Self::RespiratorySystem),
            b'K' => Some(Self::DigestiveSystem),
            b'L' => Some(Self::SkinAndSubcutaneous),
            b'M' => Some(Self::MusculoskeletalAndConnective),
            b'N' => Some(Self::GenitourinarySystem),
            b'O' => Some(Self::PregnancyChildbirthPuerperium),
            b'P' => Some(Self::PerinatalPeriod),
            b'Q' => Some(Self::CongenitalMalformations),
            b'R' => Some(Self::SymptomsAndAbnormalFindings),
            b'S' | b'T' => Some(Self::InjuryPoisoningExternalCauses),
            b'V' | b'W' | b'X' | b'Y' => Some(Self::ExternalCausesMorbidityMortality),
            b'Z' => Some(Self::HealthStatusFactors),
            b'U' => Some(Self::SpecialPurposes),
            _ => None,
        }
    }

    /// Retorna o numeral romano oficial do capítulo (ex: "IX").
    pub fn roman_numeral(&self) -> &'static str {
        match self {
            Self::InfectiousAndParasitic => "I",
            Self::Neoplasms => "II",
            Self::BloodAndImmune => "III",
            Self::EndocrineNutritionalMetabolic => "IV",
            Self::MentalAndBehavioural => "V",
            Self::NervousSystem => "VI",
            Self::EyeAndAdnexa => "VII",
            Self::EarAndMastoid => "VIII",
            Self::CirculatorySystem => "IX",
            Self::RespiratorySystem => "X",
            Self::DigestiveSystem => "XI",
            Self::SkinAndSubcutaneous => "XII",
            Self::MusculoskeletalAndConnective => "XIII",
            Self::GenitourinarySystem => "XIV",
            Self::PregnancyChildbirthPuerperium => "XV",
            Self::PerinatalPeriod => "XVI",
            Self::CongenitalMalformations => "XVII",
            Self::SymptomsAndAbnormalFindings => "XVIII",
            Self::InjuryPoisoningExternalCauses => "XIX",
            Self::ExternalCausesMorbidityMortality => "XX",
            Self::HealthStatusFactors => "XXI",
            Self::SpecialPurposes => "XXII",
        }
    }

    /// Retorna o título em português do capítulo segundo o DATASUS / OMS.
    pub fn title_pt(&self) -> &'static str {
        match self {
            Self::InfectiousAndParasitic => "Algumas doenças infecciosas e parasitárias",
            Self::Neoplasms => "Neoplasias (tumores)",
            Self::BloodAndImmune => "Doenças do sangue e dos órgãos hematopoéticos",
            Self::EndocrineNutritionalMetabolic => "Doenças endócrinas, nutricionais e metabólicas",
            Self::MentalAndBehavioural => "Transtornos mentais e comportamentais",
            Self::NervousSystem => "Doenças do sistema nervoso",
            Self::EyeAndAdnexa => "Doenças do olho e anexos",
            Self::EarAndMastoid => "Doenças do ouvido e da apófise mastóide",
            Self::CirculatorySystem => "Doenças do aparelho circulatório",
            Self::RespiratorySystem => "Doenças do aparelho respiratório",
            Self::DigestiveSystem => "Doenças do aparelho digestivo",
            Self::SkinAndSubcutaneous => "Doenças da pele e do tecido subcutâneo",
            Self::MusculoskeletalAndConnective => {
                "Doenças do sistema osteomuscular e tecido conjuntivo"
            }
            Self::GenitourinarySystem => "Doenças do aparelho geniturinário",
            Self::PregnancyChildbirthPuerperium => "Gravidez, parto e puerpério",
            Self::PerinatalPeriod => "Algumas afecções originadas no período perinatal",
            Self::CongenitalMalformations => {
                "Malformações congênitas, deformidades e anomalias cromossômicas"
            }
            Self::SymptomsAndAbnormalFindings => {
                "Sintomas, sinais e achados anormais de exames clínicos e laboratoriais"
            }
            Self::InjuryPoisoningExternalCauses => {
                "Lesões, envenenamento e consequências de causas externas"
            }
            Self::ExternalCausesMorbidityMortality => "Causas externas de morbidade e mortalidade",
            Self::HealthStatusFactors => {
                "Fatores que influenciam o estado de saúde e o contato com serviços de saúde"
            }
            Self::SpecialPurposes => "Códigos para propósitos especiais",
        }
    }
}

/// Estrutura de zero-alocação para validação e navegação de códigos CID-10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icd10Code<'a> {
    raw: &'a str,
    chapter: Icd10Chapter,
}

impl<'a> Icd10Code<'a> {
    /// Analisa e valida uma referência de string para código CID-10 sem alocação no heap.
    pub fn parse(raw: &'a str) -> Option<Self> {
        let chapter = Icd10Chapter::from_code(raw)?;
        Some(Self { raw, chapter })
    }

    /// Retorna o código bruto fornecido.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.raw
    }

    /// Retorna o capítulo canônico da OMS correspondente.
    #[inline]
    pub fn chapter(&self) -> Icd10Chapter {
        self.chapter
    }

    /// Retorna a categoria de 3 caracteres (ex: "I10").
    #[inline]
    pub fn category(&self) -> &'a str {
        let trimmed = self.raw.trim();
        if trimmed.len() >= 3 {
            &trimmed[0..3]
        } else {
            trimmed
        }
    }

    /// Classifica o código segundo os grupos CSAP (Condições Sensíveis à Atenção Primária).
    #[inline]
    pub fn csap_group(&self) -> Option<CsapGroup> {
        classify_cid10(self.raw)
    }

    /// Valida a consistência fisiológica/biológica deste código contra sexo e idade.
    pub fn validate_biological_consistency(
        &self,
        sex: BiologicalSex,
        age_years: u16,
    ) -> Result<(), PortError> {
        let cat = self.category();

        // 1. Causas estritamente femininas (Capítulo XV ou Neoplasias/Doenças ginecológicas)
        if (self.chapter == Icd10Chapter::PregnancyChildbirthPuerperium
            || matches!(
                cat,
                "C51" | "C52" | "C53" | "C54" | "C55" | "C56" | "C57" | "C58" | "N70" | "N71"
            ))
            && sex == BiologicalSex::Male
        {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: CID '{}' atribuído a indivíduo do sexo masculino",
                self.raw
            )));
        }

        // 2. Causas estritamente masculinas
        if matches!(
            cat,
            "C60" | "C61" | "C62" | "C63" | "N40" | "N41" | "N42" | "N43" | "N44" | "N45"
        ) && sex == BiologicalSex::Female
        {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: CID '{}' atribuído a indivíduo do sexo feminino",
                self.raw
            )));
        }

        // 3. Causas perinatais em indivíduos > 1 ano
        if self.chapter == Icd10Chapter::PerinatalPeriod && age_years > 1 {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: Causa perinatal CID '{}' atribuída a indivíduo de {age_years} anos",
                self.raw
            )));
        }

        // 4. Causas tipicamente senis em crianças < 15 anos
        if matches!(cat, "G30" | "F00" | "F01" | "F03") && age_years < 15 {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: Causa neurodegenerativa senil CID '{}' atribuída a criança de {age_years} anos",
                self.raw
            )));
        }

        Ok(())
    }
}

/// Sexo biológico para verificação de consistência.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiologicalSex {
    /// Masculino
    Male,
    /// Feminino
    Female,
    /// Desconhecido ou Indeterminado
    Unknown,
}

impl BiologicalSex {
    /// Converte um caractere ou string de sexo para o enum tipado.
    #[must_use]
    pub fn from_str_lenient(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "M" | "MALE" | "1" => Self::Male,
            "F" | "FEMALE" | "2" => Self::Female,
            _ => Self::Unknown,
        }
    }
}

/// Registro de inconsistência ontológica detectada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiologicalInconsistency {
    /// Incompatibilidade biológica entre causa médica e sexo masculino.
    FemaleOnlyCauseAssignedToMale { icd: String },
    /// Incompatibilidade biológica entre causa médica e sexo feminino.
    MaleOnlyCauseAssignedToFemale { icd: String },
    /// Causa tipicamente neonatal ou perinatal registrada em idade avançada.
    PerinatalCauseInAdult { icd: String, age_years: u16 },
    /// Causa tipicamente degenerativa ou senil registrada em idade pediátrica.
    SenileCauseInChildhood { icd: String, age_years: u16 },
}

/// Motor de mapeamento e validação universal de ontologias médicas.
#[derive(Debug, Default, Clone)]
pub struct MedicalOntologyHarmonizer;

static ICD10_TO_ICD11_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static ICD10_TO_SNOMED_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static ICD9_TO_ICD10_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static ICD10_TO_ICD9_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

impl MedicalOntologyHarmonizer {
    /// Cria uma nova instância do harmonizador.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn get_icd11_map() -> &'static HashMap<&'static str, &'static str> {
        ICD10_TO_ICD11_MAP.get_or_init(|| {
            HashMap::from([
                // Doenças Cardiovasculares e Hipertensão
                ("I10", "BA00"),
                ("I11", "BA01"),
                ("I20", "BA40"),
                ("I21", "BA41"),
                ("I50", "BD10"),
                ("I64", "8B20"),
                // Doenças Metabólicas e Endócrinas
                ("E10", "5A10"),
                ("E11", "5A11"),
                ("E14", "5A14"),
                ("E40", "5B50"),
                ("E66", "5B81"),
                // Doenças Respiratórias
                ("J18", "CA40"),
                ("J44", "CA22"),
                ("J45", "CA23"),
                // Doenças Infecciosas
                ("A09", "1A40"),
                ("A15", "1B10"),
                ("A90", "1D20"),
                ("B20", "1C60"),
                // Neoplasias
                ("C34", "2C25"),
                ("C50", "2C60"),
                ("C53", "2C77"),
                ("C61", "2C82"),
                // Causas Perinatais
                ("P07", "KA21"),
                ("P22", "KB23"),
                // Causas Maternas
                ("O00", "JA00"),
                ("O14", "JA21"),
                ("O72", "JA43"),
            ])
        })
    }

    fn get_snomed_map() -> &'static HashMap<&'static str, &'static str> {
        ICD10_TO_SNOMED_MAP.get_or_init(|| {
            HashMap::from([
                ("I10", "38341003"),  // Essential hypertension
                ("I21", "22298006"),  // Myocardial infarction
                ("I50", "84114007"),  // Heart failure
                ("I64", "230690007"), // Stroke
                ("E11", "44054006"),  // Type 2 diabetes mellitus
                ("J45", "195967001"), // Asthma
                ("J18", "233604007"), // Pneumonia
                ("C50", "254837009"), // Malignant neoplasm of breast
                ("C61", "254900004"), // Malignant neoplasm of prostate
            ])
        })
    }

    fn get_icd9_to_icd10_map() -> &'static HashMap<&'static str, &'static str> {
        ICD9_TO_ICD10_MAP.get_or_init(|| {
            HashMap::from([
                // Infecciosas
                ("009", "A09"), // Diarreia infecciosa
                ("010", "A15"), // Tuberculose pulmonar
                ("036", "A39"), // Infecção meningocócica
                ("042", "B20"), // Doença pelo HIV
                ("061", "A90"), // Dengue
                // Neoplasias
                ("150", "C15"), // Esôfago
                ("151", "C16"), // Estômago
                ("153", "C18"), // Cólon
                ("162", "C34"), // Brônquios e pulmão
                ("174", "C50"), // Mama feminina
                ("180", "C53"), // Colo do útero
                ("185", "C61"), // Próstata
                ("204", "C91"), // Leucemia linfoide
                // Endócrinas
                ("250", "E14"), // Diabetes mellitus
                ("260", "E40"), // Kwashiorkor / Desnutrição
                // Cardiovasculares
                ("401", "I10"), // Hipertensão essencial
                ("410", "I21"), // Infarto agudo do miocárdio
                ("413", "I20"), // Angina pectoris
                ("428", "I50"), // Insuficiência cardíaca
                ("436", "I64"), // AVC / Doença cerebrovascular aguda
                ("440", "I70"), // Aterosclerose
                // Respiratórias
                ("486", "J18"), // Pneumonia
                ("491", "J44"), // Bronquite crônica / DPOC
                ("493", "J45"), // Asma
                // Digestivas
                ("531", "K25"), // Úlcera gástrica
                ("540", "K35"), // Apendicite aguda
                ("571", "K70"), // Doença hepática crônica / Cirrose
                // Causas Externas
                ("E810", "V89"), // Acidente de trânsito
                ("E819", "V89"),
                ("E950", "X60"), // Suicídio e autolesão
                ("E960", "X85"), // Homicídio e agressão
                ("E965", "X95"), // Agressão por arma de fogo
            ])
        })
    }

    fn get_icd10_to_icd9_map() -> &'static HashMap<&'static str, &'static str> {
        ICD10_TO_ICD9_MAP.get_or_init(|| {
            HashMap::from([
                ("A09", "009"),
                ("A15", "010"),
                ("A39", "036"),
                ("B20", "042"),
                ("A90", "061"),
                ("C15", "150"),
                ("C16", "151"),
                ("C18", "153"),
                ("C34", "162"),
                ("C50", "174"),
                ("C53", "180"),
                ("C61", "185"),
                ("C91", "204"),
                ("E10", "250"),
                ("E11", "250"),
                ("E14", "250"),
                ("E40", "260"),
                ("I10", "401"),
                ("I20", "413"),
                ("I21", "410"),
                ("I50", "428"),
                ("I64", "436"),
                ("I70", "440"),
                ("J18", "486"),
                ("J44", "491"),
                ("J45", "493"),
                ("K25", "531"),
                ("K35", "540"),
                ("K70", "571"),
                ("V89", "E819"),
                ("X60", "E950"),
                ("X85", "E960"),
                ("X95", "E965"),
            ])
        })
    }

    /// Mapeia um código da CID-10 para a CID-11.
    ///
    /// Aceita códigos com ou sem ponto (ex.: `"I10"`, `"I10.0"`).
    #[must_use]
    pub fn map_icd10_to_icd11(&self, icd10: &str) -> Option<String> {
        let cleaned = icd10.trim().to_uppercase();
        let prefix = if cleaned.len() >= 3 {
            &cleaned[0..3]
        } else {
            &cleaned
        };

        Self::get_icd11_map().get(prefix).map(|&v| v.to_string())
    }

    /// Mapeia um código da CID-10 para o identificador de conceito SNOMED-CT (SCTID).
    #[must_use]
    pub fn map_icd10_to_snomed(&self, icd10: &str) -> Option<String> {
        let cleaned = icd10.trim().to_uppercase();
        let prefix = if cleaned.len() >= 3 {
            &cleaned[0..3]
        } else {
            &cleaned
        };

        Self::get_snomed_map().get(prefix).map(|&v| v.to_string())
    }

    /// Mapeia um código histórico da CID-9 para a CID-10 correspondente.
    ///
    /// Aceita códigos numéricos puros ou com prefixo 'E' (Causas Externas).
    #[must_use]
    pub fn map_icd9_to_icd10(&self, icd9: &str) -> Option<String> {
        let cleaned = icd9.trim().to_uppercase();
        let prefix = if cleaned.starts_with('E') && cleaned.len() >= 4 {
            &cleaned[0..4]
        } else if cleaned.len() >= 3 {
            &cleaned[0..3]
        } else {
            &cleaned
        };

        Self::get_icd9_to_icd10_map()
            .get(prefix)
            .map(|&v| v.to_string())
    }

    /// Mapeia um código da CID-10 para a representação histórica equivalente na CID-9.
    #[must_use]
    pub fn map_icd10_to_icd9(&self, icd10: &str) -> Option<String> {
        let cleaned = icd10.trim().to_uppercase();
        let prefix = if cleaned.len() >= 3 {
            &cleaned[0..3]
        } else {
            &cleaned
        };

        Self::get_icd10_to_icd9_map()
            .get(prefix)
            .map(|&v| v.to_string())
    }

    /// Valida a consistência biológica de um evento médico segundo idade e sexo.
    ///
    /// Retorna `Ok(())` se os parâmetros forem compatíveis ou `Err(PortError::DomainValidation)`
    /// com a violação fisiológica encontrada.
    pub fn validate_biological_consistency(
        &self,
        icd10: &str,
        sex: BiologicalSex,
        age_years: u16,
    ) -> Result<(), PortError> {
        if let Some(parsed) = Icd10Code::parse(icd10) {
            parsed.validate_biological_consistency(sex, age_years)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icd10_to_icd11_cross_mapping() {
        let harmonizer = MedicalOntologyHarmonizer::new();

        assert_eq!(
            harmonizer.map_icd10_to_icd11("I10"),
            Some("BA00".to_string())
        );
        assert_eq!(
            harmonizer.map_icd10_to_icd11("E11.9"),
            Some("5A11".to_string())
        );
        assert_eq!(
            harmonizer.map_icd10_to_icd11("J45"),
            Some("CA23".to_string())
        );
        assert_eq!(harmonizer.map_icd10_to_icd11("XYZ99"), None);
    }

    #[test]
    fn test_icd10_to_snomed() {
        let harmonizer = MedicalOntologyHarmonizer::new();

        assert_eq!(
            harmonizer.map_icd10_to_snomed("I21.0"),
            Some("22298006".to_string())
        );
        assert_eq!(
            harmonizer.map_icd10_to_snomed("E11"),
            Some("44054006".to_string())
        );
    }

    #[test]
    fn test_biological_consistency_sex_validation() {
        let harmonizer = MedicalOntologyHarmonizer::new();

        // Parto ou gravidez em homem -> Erro
        let res_preg = harmonizer.validate_biological_consistency("O80", BiologicalSex::Male, 25);
        assert!(res_preg.is_err());

        // Câncer de colo de útero em homem -> Erro
        let res_cervix =
            harmonizer.validate_biological_consistency("C53.9", BiologicalSex::Male, 40);
        assert!(res_cervix.is_err());

        // Câncer de próstata em mulher -> Erro
        let res_prostate =
            harmonizer.validate_biological_consistency("C61", BiologicalSex::Female, 65);
        assert!(res_prostate.is_err());

        // Parto em mulher -> OK
        let res_valid =
            harmonizer.validate_biological_consistency("O80", BiologicalSex::Female, 28);
        assert!(res_valid.is_ok());
    }

    #[test]
    fn test_biological_consistency_age_validation() {
        let harmonizer = MedicalOntologyHarmonizer::new();

        // Causa perinatal em adulto de 30 anos -> Erro
        let res_perinatal =
            harmonizer.validate_biological_consistency("P07.3", BiologicalSex::Male, 30);
        assert!(res_perinatal.is_err());

        // Causa perinatal em bebê de 0 anos -> OK
        let res_infant =
            harmonizer.validate_biological_consistency("P07.3", BiologicalSex::Male, 0);
        assert!(res_infant.is_ok());

        // Alzheimer em criança de 5 anos -> Erro
        let res_alz = harmonizer.validate_biological_consistency("G30.9", BiologicalSex::Female, 5);
        assert!(res_alz.is_err());

        // Alzheimer em idosa de 80 anos -> OK
        let res_senile =
            harmonizer.validate_biological_consistency("G30.9", BiologicalSex::Female, 80);
        assert!(res_senile.is_ok());
    }

    #[test]
    fn test_icd10_chapter_classification() {
        assert_eq!(
            Icd10Chapter::from_code("A09"),
            Some(Icd10Chapter::InfectiousAndParasitic)
        );
        assert_eq!(
            Icd10Chapter::from_code("C50.9"),
            Some(Icd10Chapter::Neoplasms)
        );
        assert_eq!(
            Icd10Chapter::from_code("D50"),
            Some(Icd10Chapter::BloodAndImmune)
        );
        assert_eq!(
            Icd10Chapter::from_code("E11"),
            Some(Icd10Chapter::EndocrineNutritionalMetabolic)
        );
        assert_eq!(
            Icd10Chapter::from_code("I10"),
            Some(Icd10Chapter::CirculatorySystem)
        );
        assert_eq!(
            Icd10Chapter::from_code("J45"),
            Some(Icd10Chapter::RespiratorySystem)
        );
        assert_eq!(
            Icd10Chapter::from_code("O80"),
            Some(Icd10Chapter::PregnancyChildbirthPuerperium)
        );
        assert_eq!(
            Icd10Chapter::from_code("P07"),
            Some(Icd10Chapter::PerinatalPeriod)
        );
        assert_eq!(
            Icd10Chapter::from_code("V89"),
            Some(Icd10Chapter::ExternalCausesMorbidityMortality)
        );
        assert_eq!(
            Icd10Chapter::from_code("U07.1"),
            Some(Icd10Chapter::SpecialPurposes)
        );
        assert_eq!(Icd10Chapter::from_code("123"), None);

        let ch = Icd10Chapter::CirculatorySystem;
        assert_eq!(ch.roman_numeral(), "IX");
        assert_eq!(ch.title_pt(), "Doenças do aparelho circulatório");
    }

    #[test]
    fn test_icd10_code_struct() {
        let code = Icd10Code::parse("J45.0").unwrap();
        assert_eq!(code.chapter(), Icd10Chapter::RespiratorySystem);
        assert_eq!(code.category(), "J45");
        assert_eq!(code.csap_group(), Some(CsapGroup::Asma));
    }
}
