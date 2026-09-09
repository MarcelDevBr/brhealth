// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Harmonizador e Validador da Tabela de Procedimentos do SUS (SIGTAP).
//!
//! Implementa validação estrutural do código de 10 dígitos do Sistema de Gerenciamento
//! da Tabela de Procedimentos, Medicamentos e OPM do SUS (SIGTAP) e rotinas para
//! identificação de **Eventos Sentinela** e **Procedimentos de Alta Complexidade**
//! (notadamente amputações de membros inferiores, hemodiálise e cirurgias evitáveis).
//!
//! # Estrutura do Código SIGTAP (10 Dígitos)
//! O código do procedimento segue a divisão hierárquica oficial do Ministério da Saúde:
//!
//! $$\text{SIGTAP} = \underbrace{d_1 d_2}_{\text{Grupo}} \cdot \underbrace{d_3 d_4}_{\text{Subgrupo}} \cdot \underbrace{d_5 d_6}_{\text{Forma Org.}} \cdot \underbrace{d_7 d_8 d_9}_{\text{Sequencial}} - \underbrace{d_{10}}_{\text{DV}}$$
//!
//! - **Grupo (01 a 08)**:
//!   - `01`: Ações de promoção e prevenção em saúde
//!   - `02`: Procedimentos com finalidade diagnóstica
//!   - `03`: Procedimentos clínicos
//!   - `04`: Procedimentos cirúrgicos
//!   - `05`: Transplantes de órgãos, tecidos e células
//!   - `06`: Medicamentos
//!   - `07`: Órteses, próteses e materiais especiais (OPM)
//!   - `08`: Ações complementares da atenção à saúde

use serde::{Deserialize, Serialize};

use crate::domain::ports::outbound::PortError;

/// Procedimentos de Amputação de Membros Inferiores (Complicações graves de Diabetes Mellitus).
pub const PROCEDIMENTO_AMPUTACAO_ARTELHOS: &str = "0407040080";
pub const PROCEDIMENTO_AMPUTACAO_PE_TARSO: &str = "0407040098";
pub const PROCEDIMENTO_AMPUTACAO_PERNA: &str = "0407040101";
pub const PROCEDIMENTO_AMPUTACAO_COXA: &str = "0407040110";

/// Procedimentos de Terapia Renal Substitutiva (Hemodiálise contínua).
pub const PROCEDIMENTO_HEMODIALISE_CONTINUA: &str = "0305010107";
pub const PROCEDIMENTO_HEMODIALISE_MAX_3_SEM: &str = "0305010115";

/// Representação estruturada e tipada de um código SIGTAP do SUS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigtapCode {
    /// Grupo temático (2 dígitos: 01 a 08).
    pub group: u8,
    /// Subgrupo temático (2 dígitos).
    pub subgroup: u8,
    /// Forma de organização assistencial (2 dígitos).
    pub form_of_organization: u8,
    /// Número sequencial específico (3 dígitos).
    pub sequential: u16,
    /// Dígito Verificador Módulo 10 (1 dígito).
    pub dv: u8,
}

impl SigtapCode {
    /// Formata o código SIGTAP na representação canônica sem separadores (10 dígitos).
    #[must_use]
    pub fn to_string_unformatted(&self) -> String {
        format!(
            "{:02}{:02}{:02}{:03}{}",
            self.group, self.subgroup, self.form_of_organization, self.sequential, self.dv
        )
    }

    /// Formata o código SIGTAP no padrão visual do DATASUS (`04.07.04.010-1`).
    #[must_use]
    pub fn to_string_formatted(&self) -> String {
        format!(
            "{:02}.{:02}.{:02}.{:03}-{}",
            self.group, self.subgroup, self.form_of_organization, self.sequential, self.dv
        )
    }
}

/// Analisa e valida a estrutura de um código SIGTAP de 10 dígitos.
///
/// Aceita códigos com pontuação (`04.07.04.010-1`) ou apenas dígitos (`0407040101`).
pub fn parse_sigtap_code(raw: &str) -> Result<SigtapCode, PortError> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() != 10 {
        return Err(PortError::ValidationError(format!(
            "Código SIGTAP '{raw}' inválido: esperado 10 dígitos, encontrado {}",
            digits.len()
        )));
    }

    let group = digits[0..2]
        .parse::<u8>()
        .map_err(|_| PortError::ValidationError("Falha ao analisar grupo SIGTAP".into()))?;

    let subgroup = digits[2..4]
        .parse::<u8>()
        .map_err(|_| PortError::ValidationError("Falha ao analisar subgrupo SIGTAP".into()))?;

    let form_of_organization = digits[4..6].parse::<u8>().map_err(|_| {
        PortError::ValidationError("Falha ao analisar forma de organização SIGTAP".into())
    })?;

    let sequential = digits[6..9]
        .parse::<u16>()
        .map_err(|_| PortError::ValidationError("Falha ao analisar sequencial SIGTAP".into()))?;

    let dv = digits[9..10]
        .parse::<u8>()
        .map_err(|_| PortError::ValidationError("Falha ao analisar DV SIGTAP".into()))?;

    Ok(SigtapCode {
        group,
        subgroup,
        form_of_organization,
        sequential,
        dv,
    })
}

/// Identifica se o código do procedimento refere-se a uma cirurgia de amputação de membro inferior.
///
/// Monitora eventos evitáveis de descompensação vascular periférica e pé diabético
/// (Portaria MS/SAS nº 221/2008).
#[must_use]
pub fn is_amputation_procedure(code: &str) -> bool {
    let cleaned: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    matches!(
        cleaned.as_str(),
        PROCEDIMENTO_AMPUTACAO_ARTELHOS
            | PROCEDIMENTO_AMPUTACAO_PE_TARSO
            | PROCEDIMENTO_AMPUTACAO_PERNA
            | PROCEDIMENTO_AMPUTACAO_COXA
    )
}

/// Identifica se o procedimento é uma sessão de terapia renal substitutiva (hemodiálise).
#[must_use]
pub fn is_dialysis_procedure(code: &str) -> bool {
    let cleaned: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    matches!(
        cleaned.as_str(),
        PROCEDIMENTO_HEMODIALISE_CONTINUA | PROCEDIMENTO_HEMODIALISE_MAX_3_SEM
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sigtap_valid() {
        let sigtap = parse_sigtap_code("04.07.04.010-1").unwrap();
        assert_eq!(sigtap.group, 4);
        assert_eq!(sigtap.subgroup, 7);
        assert_eq!(sigtap.form_of_organization, 4);
        assert_eq!(sigtap.sequential, 10);
        assert_eq!(sigtap.dv, 1);
        assert_eq!(sigtap.to_string_unformatted(), "0407040101");
    }

    #[test]
    fn test_amputation_identification() {
        assert!(is_amputation_procedure("0407040101")); // Perna
        assert!(is_amputation_procedure("04.07.04.008-0")); // Artelhos
        assert!(!is_amputation_procedure("0305010107")); // Hemodiálise
    }

    #[test]
    fn test_dialysis_identification() {
        assert!(is_dialysis_procedure("0305010107"));
        assert!(is_dialysis_procedure("03.05.01.011-5"));
        assert!(!is_dialysis_procedure("0407040101"));
    }
}
