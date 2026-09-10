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

use crate::domain::ports::outbound::PortError;

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

        Self::get_icd9_to_icd10_map().get(prefix).map(|&v| v.to_string())
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

        Self::get_icd10_to_icd9_map().get(prefix).map(|&v| v.to_string())
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
        let cleaned = icd10.trim().to_uppercase();
        if cleaned.is_empty() {
            return Ok(());
        }

        let prefix_letter = cleaned.chars().next().unwrap_or(' ');
        let prefix3 = if cleaned.len() >= 3 {
            &cleaned[0..3]
        } else {
            &cleaned
        };

        // 1. Causas estritamente femininas
        // O00-O99 (Gravidez, parto e puerpério), C51-C58 (Neoplasias genitais femininas)
        if (prefix_letter == 'O'
            || matches!(
                prefix3,
                "C51" | "C52" | "C53" | "C54" | "C55" | "C56" | "C57" | "C58" | "N70" | "N71"
            ))
            && sex == BiologicalSex::Male
        {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: CID '{cleaned}' atribuído a indivíduo do sexo masculino"
            )));
        }

        // 2. Causas estritamente masculinas
        // C60-C63 (Neoplasias genitais masculinas: pênis, próstata, testículo), N40-N51 (Doenças órgãos genitais masc)
        if matches!(
            prefix3,
            "C60" | "C61" | "C62" | "C63" | "N40" | "N41" | "N42" | "N43" | "N44" | "N45"
        ) && sex == BiologicalSex::Female
        {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: CID '{cleaned}' atribuído a indivíduo do sexo feminino"
            )));
        }

        // 3. Causas perinatais em indivíduos com mais de 1 ano
        if prefix_letter == 'P' && age_years > 1 {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: Causa perinatal CID '{cleaned}' atribuída a indivíduo de {age_years} anos"
            )));
        }

        // 4. Causas tipicamente senis (Alzheimer e demência degenerativa) em crianças < 15 anos
        if matches!(prefix3, "G30" | "F00" | "F01" | "F03") && age_years < 15 {
            return Err(PortError::ValidationError(format!(
                "Inconsistência biológica: Causa neurodegenerativa senil CID '{cleaned}' atribuída a criança de {age_years} anos"
            )));
        }

        Ok(())
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
}
