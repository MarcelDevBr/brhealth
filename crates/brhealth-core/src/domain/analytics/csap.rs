// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Módulo analítico de Condições Sensíveis à Atenção Primária (CSAP) e Economia da Saúde.
//!
//! Este módulo implementa a Lista Brasileira de Internações por Condições Sensíveis
//! à Atenção Primária em conformidade estrita com a **Portaria MS/SAS nº 221, de 17 de abril de 2008**,
//! além de métricas bioestatísticas de taxas populacionais, custos evitáveis e Retorno
//! sobre o Investimento (ROI) em Saúde Coletiva na Atenção Básica.

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanBuilder, Float32Array, Float64Array, Int32Array, Int64Array,
    RecordBatch, StringArray, StringBuilder, UInt8Array, UInt8Builder, UInt16Array, UInt32Array,
    UInt64Array,
};
use arrow::datatypes::{DataType, Field, Schema};

use crate::domain::ports::outbound::PortError;

/// Os 19 grupos canônicos de causas de CSAP conforme Portaria MS/SAS nº 221/2008.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum CsapGroup {
    /// 1. Doenças preveníveis por imunização e condições sensíveis
    Imunopreveniveis = 1,
    /// 2. Gastroenterites infecciosas e complicações
    Gastroenterites = 2,
    /// 3. Anemia
    Anemia = 3,
    /// 4. Deficiências nutricionais
    DeficienciasNutricionais = 4,
    /// 5. Infecções de ouvido, nariz e garganta
    InfeccoesOuvidoNarizGarganta = 5,
    /// 6. Pneumonias bacterianas
    PneumoniasBacterianas = 6,
    /// 7. Asma
    Asma = 7,
    /// 8. Doenças pulmonares (DPOC, bronquiectasia)
    DoencasPulmonares = 8,
    /// 9. Hipertensão arterial sistêmica
    Hipertensao = 9,
    /// 10. Angina pectoris
    Angina = 10,
    /// 11. Insuficiência cardíaca
    InsuficienciaCardiaca = 11,
    /// 12. Doenças cerebrovasculares
    DoencasCerebrovasculares = 12,
    /// 13. Diabetes mellitus
    DiabetesMellitus = 13,
    /// 14. Epilepsias
    Epilepsias = 14,
    /// 15. Infecção do trato urinário
    InfeccaoTratoUrinario = 15,
    /// 16. Infecção da pele e tecido subcutâneo
    InfeccoesPele = 16,
    /// 17. Doença inflamatória dos órgãos pélvicos femininos
    DoencaInflamatoriaPelvica = 17,
    /// 18. Úlcera gastrointestinal
    UlceraGastrointestinal = 18,
    /// 19. Doenças relacionadas ao pré-natal e parto
    DoencasPreNatalParto = 19,
}

impl CsapGroup {
    /// Retorna o identificador numérico oficial (1 a 19).
    #[inline]
    pub fn id(&self) -> u8 {
        *self as u8
    }

    /// Retorna o título descritivo do grupo em português.
    pub fn name(&self) -> &'static str {
        match self {
            CsapGroup::Imunopreveniveis => {
                "Doenças preveníveis por imunização e condições sensíveis"
            }
            CsapGroup::Gastroenterites => "Gastroenterites infecciosas e complicações",
            CsapGroup::Anemia => "Anemia",
            CsapGroup::DeficienciasNutricionais => "Deficiências nutricionais",
            CsapGroup::InfeccoesOuvidoNarizGarganta => "Infecções de ouvido, nariz e garganta",
            CsapGroup::PneumoniasBacterianas => "Pneumonias bacterianas",
            CsapGroup::Asma => "Asma",
            CsapGroup::DoencasPulmonares => "Doenças pulmonares",
            CsapGroup::Hipertensao => "Hipertensão",
            CsapGroup::Angina => "Angina",
            CsapGroup::InsuficienciaCardiaca => "Insuficiência cardíaca",
            CsapGroup::DoencasCerebrovasculares => "Doenças cerebrovasculares",
            CsapGroup::DiabetesMellitus => "Diabetes mellitus",
            CsapGroup::Epilepsias => "Epilepsias",
            CsapGroup::InfeccaoTratoUrinario => "Infecção do trato urinário",
            CsapGroup::InfeccoesPele => "Infecção da pele e tecido subcutâneo",
            CsapGroup::DoencaInflamatoriaPelvica => {
                "Doença inflamatória dos órgãos pélvicos femininos"
            }
            CsapGroup::UlceraGastrointestinal => "Úlcera gastrointestinal",
            CsapGroup::DoencasPreNatalParto => "Doenças relacionadas ao pré-natal e parto",
        }
    }

    /// Converte um ID numérico (1..=19) no respectivo enum `CsapGroup`.
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(CsapGroup::Imunopreveniveis),
            2 => Some(CsapGroup::Gastroenterites),
            3 => Some(CsapGroup::Anemia),
            4 => Some(CsapGroup::DeficienciasNutricionais),
            5 => Some(CsapGroup::InfeccoesOuvidoNarizGarganta),
            6 => Some(CsapGroup::PneumoniasBacterianas),
            7 => Some(CsapGroup::Asma),
            8 => Some(CsapGroup::DoencasPulmonares),
            9 => Some(CsapGroup::Hipertensao),
            10 => Some(CsapGroup::Angina),
            11 => Some(CsapGroup::InsuficienciaCardiaca),
            12 => Some(CsapGroup::DoencasCerebrovasculares),
            13 => Some(CsapGroup::DiabetesMellitus),
            14 => Some(CsapGroup::Epilepsias),
            15 => Some(CsapGroup::InfeccaoTratoUrinario),
            16 => Some(CsapGroup::InfeccoesPele),
            17 => Some(CsapGroup::DoencaInflamatoriaPelvica),
            18 => Some(CsapGroup::UlceraGastrointestinal),
            19 => Some(CsapGroup::DoencasPreNatalParto),
            _ => None,
        }
    }
}

/// Classifica um código CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008.
///
/// O código CID-10 de entrada pode conter ou não pontuação (ex: `"J45"`, `"J45.0"`, `"J450"`).
///
/// # Exemplos
///
/// ```
/// use brhealth_core::domain::analytics::csap::{classify_cid10, CsapGroup};
///
/// assert_eq!(classify_cid10("J45"), Some(CsapGroup::Asma));
/// assert_eq!(classify_cid10("I10"), Some(CsapGroup::Hipertensao));
/// assert_eq!(classify_cid10("E119"), Some(CsapGroup::DiabetesMellitus));
/// assert_eq!(classify_cid10("S060"), None); // Trauma não é CSAP
/// ```
pub fn classify_cid10(cid: &str) -> Option<CsapGroup> {
    let trimmed = cid.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Normalizar removendo pontos (ex: "J45.0" -> "J450") em buffer de stack sem alocações
    let mut buf = [0u8; 8];
    let mut len = 0;
    for b in trimmed.bytes() {
        if b.is_ascii_alphanumeric() {
            if len >= 8 {
                return None;
            }
            buf[len] = b.to_ascii_uppercase();
            len += 1;
        }
    }

    if len < 3 {
        return None;
    }

    let full = match std::str::from_utf8(&buf[..len]) {
        Ok(s) => s,
        Err(_) => return None,
    };
    let prefix3 = &full[..3];

    // 1. Imunopreveníveis: A33-A37, A95, B16, B05, B06, B26, G000, A170, A19 (exc A192)
    if matches!(
        prefix3,
        "A33" | "A34" | "A35" | "A36" | "A37" | "A95" | "B16" | "B05" | "B06" | "B26"
    ) || full.starts_with("G000")
        || full.starts_with("A170")
        || (prefix3 == "A19" && !full.starts_with("A192"))
    {
        return Some(CsapGroup::Imunopreveniveis);
    }

    // 2. Gastroenterites: A00-A09, E86
    if ("A00"..="A09").contains(&prefix3) || prefix3 == "E86" {
        return Some(CsapGroup::Gastroenterites);
    }

    // 3. Anemia: D50
    if prefix3 == "D50" {
        return Some(CsapGroup::Anemia);
    }

    // 4. Deficiências Nutricionais: E40-E46, E50-E64
    if ("E40"..="E46").contains(&prefix3) || ("E50"..="E64").contains(&prefix3) {
        return Some(CsapGroup::DeficienciasNutricionais);
    }

    // 5. Infecções de ouvido, nariz e garganta: H66, J00-J03, J06, J31
    if prefix3 == "H66"
        || ("J00"..="J03").contains(&prefix3)
        || prefix3 == "J06"
        || prefix3 == "J31"
    {
        return Some(CsapGroup::InfeccoesOuvidoNarizGarganta);
    }

    // 6. Pneumonias bacterianas: J13, J14, J15.3, J15.4, J15.8, J15.9, J18.1
    if prefix3 == "J13"
        || prefix3 == "J14"
        || full.starts_with("J153")
        || full.starts_with("J154")
        || full.starts_with("J158")
        || full.starts_with("J159")
        || full.starts_with("J181")
    {
        return Some(CsapGroup::PneumoniasBacterianas);
    }

    // 7. Asma: J45, J46
    if prefix3 == "J45" || prefix3 == "J46" {
        return Some(CsapGroup::Asma);
    }

    // 8. Doenças pulmonares: J20, J21, J40-J44, J47
    if prefix3 == "J20"
        || prefix3 == "J21"
        || ("J40"..="J44").contains(&prefix3)
        || prefix3 == "J47"
    {
        return Some(CsapGroup::DoencasPulmonares);
    }

    // 9. Hipertensão: I10, I11
    if prefix3 == "I10" || prefix3 == "I11" {
        return Some(CsapGroup::Hipertensao);
    }

    // 10. Angina: I20
    if prefix3 == "I20" {
        return Some(CsapGroup::Angina);
    }

    // 11. Insuficiência cardíaca: I50, J81
    if prefix3 == "I50" || prefix3 == "J81" {
        return Some(CsapGroup::InsuficienciaCardiaca);
    }

    // 12. Doenças cerebrovasculares: I63-I67, I69, G45, G46
    if ("I63"..="I67").contains(&prefix3)
        || prefix3 == "I69"
        || prefix3 == "G45"
        || prefix3 == "G46"
    {
        return Some(CsapGroup::DoencasCerebrovasculares);
    }

    // 13. Diabetes mellitus: E10-E14
    if ("E10"..="E14").contains(&prefix3) {
        return Some(CsapGroup::DiabetesMellitus);
    }

    // 14. Epilepsias: G40, G41
    if prefix3 == "G40" || prefix3 == "G41" {
        return Some(CsapGroup::Epilepsias);
    }

    // 15. Infecção do trato urinário: N10-N12, N30, N34, N39.0
    if ("N10"..="N12").contains(&prefix3)
        || prefix3 == "N30"
        || prefix3 == "N34"
        || full.starts_with("N390")
    {
        return Some(CsapGroup::InfeccaoTratoUrinario);
    }

    // 16. Infecção da pele e tecido subcutâneo: A46, L01-L04, L08
    if prefix3 == "A46" || ("L01"..="L04").contains(&prefix3) || prefix3 == "L08" {
        return Some(CsapGroup::InfeccoesPele);
    }

    // 17. Doença inflamatória dos órgãos pélvicos femininos: N70-N73, N75, N76
    if ("N70"..="N73").contains(&prefix3) || prefix3 == "N75" || prefix3 == "N76" {
        return Some(CsapGroup::DoencaInflamatoriaPelvica);
    }

    // 18. Úlcera gastrointestinal: K25-K28, K92.0, K92.1, K92.2
    if ("K25"..="K28").contains(&prefix3)
        || full.starts_with("K920")
        || full.starts_with("K921")
        || full.starts_with("K922")
    {
        return Some(CsapGroup::UlceraGastrointestinal);
    }

    // 19. Doenças relacionadas ao pré-natal e parto: O15, O20.0, O23, O60, P35.0, A50, A51-A53
    if prefix3 == "O15"
        || full.starts_with("O200")
        || prefix3 == "O23"
        || prefix3 == "O60"
        || full.starts_with("P350")
        || prefix3 == "A50"
        || ("A51"..="A53").contains(&prefix3)
    {
        return Some(CsapGroup::DoencasPreNatalParto);
    }

    None
}

/// Verifica de forma direta se o código CID-10 pertence às CSAP.
#[inline]
pub fn is_csap(cid: &str) -> bool {
    classify_cid10(cid).is_some()
}

/// Métricas bioestatísticas e econômicas agregadas de CSAP.
#[derive(Debug, Clone, PartialEq)]
pub struct CsapMetrics {
    /// Total de admissões hospitalares processadas.
    pub total_admissions: u64,
    /// Total de admissões hospitalares classificadas como CSAP.
    pub csap_admissions: u64,
    /// Proporção de internações por CSAP ($\frac{\text{CSAP}}{\text{Total}}$).
    pub csap_proportion: f64,
    /// Taxa de internação por CSAP por 10.000 habitantes.
    pub csap_rate_per_10k: Option<f64>,
    /// Custo financeiro total acumulado de todas as internações (R$).
    pub total_cost: f64,
    /// Custo financeiro direto evitável associado às internações por CSAP (R$).
    pub avoidable_cost: f64,
    /// Dias totais de permanência hospitalar.
    pub total_days: u64,
    /// Dias totais de permanência hospitalar evitáveis (CSAP).
    pub avoidable_days: u64,
    /// Distribuição de contagem de internações por grupo (índice 0 corresponde ao Grupo 1).
    pub group_counts: [u64; 19],
    /// Distribuição de custos financeiros por grupo (índice 0 corresponde ao Grupo 1).
    pub group_costs: [f64; 19],
}

/// Localiza a coluna de diagnóstico principal em um `RecordBatch` verificando aliases canônicos e do DATASUS.
fn resolve_diagnosis_column(batch: &RecordBatch) -> Result<&StringArray, PortError> {
    const DIAG_CANDIDATES: &[&str] = &[
        "primary_diagnosis",
        "main_diagnosis_icd10",
        "diag_princ",
        "DIAG_PRINC",
    ];

    for &name in DIAG_CANDIDATES {
        if let Ok(idx) = batch.schema().index_of(name)
            && let Some(arr) = batch.column(idx).as_any().downcast_ref::<StringArray>()
        {
            return Ok(arr);
        }
    }

    Err(PortError::SchemaMismatch(
        "Nenhuma coluna de diagnóstico principal encontrada (esperado 'primary_diagnosis', 'main_diagnosis_icd10' ou 'DIAG_PRINC')".into(),
    ))
}

enum CostExtractor<'a> {
    F64(&'a Float64Array),
    F32(&'a Float32Array),
    I64(&'a Int64Array),
    U64(&'a UInt64Array),
    Str(&'a StringArray),
    None,
}

impl<'a> CostExtractor<'a> {
    fn resolve(batch: &'a RecordBatch) -> Self {
        const COST_CANDIDATES: &[&str] = &["total_cost", "total_paid_amount", "val_tot", "VAL_TOT"];

        for &name in COST_CANDIDATES {
            if let Ok(idx) = batch.schema().index_of(name) {
                let col = batch.column(idx);
                if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                    return Self::F64(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
                    return Self::F32(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                    return Self::I64(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<UInt64Array>() {
                    return Self::U64(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    return Self::Str(arr);
                }
            }
        }
        Self::None
    }

    #[inline]
    fn get(&self, row: usize) -> f64 {
        match self {
            Self::F64(arr) => {
                if arr.is_valid(row) {
                    arr.value(row)
                } else {
                    0.0
                }
            }
            Self::F32(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as f64
                } else {
                    0.0
                }
            }
            Self::I64(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as f64
                } else {
                    0.0
                }
            }
            Self::U64(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as f64
                } else {
                    0.0
                }
            }
            Self::Str(arr) => {
                if arr.is_valid(row) {
                    arr.value(row).trim().parse::<f64>().unwrap_or(0.0)
                } else {
                    0.0
                }
            }
            Self::None => 0.0,
        }
    }
}

enum DaysExtractor<'a> {
    U16(&'a UInt16Array),
    U8(&'a UInt8Array),
    U32(&'a UInt32Array),
    I64(&'a Int64Array),
    I32(&'a Int32Array),
    Str(&'a StringArray),
    None,
}

impl<'a> DaysExtractor<'a> {
    fn resolve(batch: &'a RecordBatch) -> Self {
        const DAYS_CANDIDATES: &[&str] = &[
            "length_of_stay",
            "length_of_stay_days",
            "dias_perm",
            "DIAS_PERM",
        ];

        for &name in DAYS_CANDIDATES {
            if let Ok(idx) = batch.schema().index_of(name) {
                let col = batch.column(idx);
                if let Some(arr) = col.as_any().downcast_ref::<UInt16Array>() {
                    return Self::U16(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<UInt8Array>() {
                    return Self::U8(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<UInt32Array>() {
                    return Self::U32(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                    return Self::I64(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                    return Self::I32(arr);
                } else if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    return Self::Str(arr);
                }
            }
        }
        Self::None
    }

    #[inline]
    fn get(&self, row: usize) -> u64 {
        match self {
            Self::U16(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as u64
                } else {
                    0
                }
            }
            Self::U8(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as u64
                } else {
                    0
                }
            }
            Self::U32(arr) => {
                if arr.is_valid(row) {
                    arr.value(row) as u64
                } else {
                    0
                }
            }
            Self::I64(arr) => {
                if arr.is_valid(row) {
                    arr.value(row).max(0) as u64
                } else {
                    0
                }
            }
            Self::I32(arr) => {
                if arr.is_valid(row) {
                    arr.value(row).max(0) as u64
                } else {
                    0
                }
            }
            Self::Str(arr) => {
                if arr.is_valid(row) {
                    arr.value(row).trim().parse::<u64>().unwrap_or(0)
                } else {
                    0
                }
            }
            Self::None => 0,
        }
    }
}

/// Enriquece um `RecordBatch` canônico do SIH (`canonical_hospital_morbidity_schema`)
/// adicionando ou atualizando as colunas colunares derivadas:
/// - `is_csap` (Boolean): indica se a internação pertence à lista de CSAP.
/// - `csap_group_id` (UInt8, nullable): identificador de 1 a 19 do grupo de CSAP.
/// - `csap_group_name` (Utf8, nullable): descrição em texto do grupo.
///
/// Suporta aliases de coluna de diagnóstico principal como `primary_diagnosis`, `main_diagnosis_icd10` e `DIAG_PRINC`.
///
/// # Erros
///
/// Retorna `PortError::SchemaMismatch` caso nenhuma coluna de diagnóstico compatível seja encontrada.
pub fn enrich_sih_batch_with_csap(batch: &RecordBatch) -> Result<RecordBatch, PortError> {
    let diag_col = resolve_diagnosis_column(batch)?;
    let num_rows = batch.num_rows();
    let mut is_csap_builder = BooleanBuilder::with_capacity(num_rows);
    let mut group_id_builder = UInt8Builder::with_capacity(num_rows);
    let mut group_name_builder = StringBuilder::with_capacity(num_rows, num_rows * 32);

    for i in 0..num_rows {
        if diag_col.is_valid(i) {
            let cid = diag_col.value(i);
            if let Some(group) = classify_cid10(cid) {
                is_csap_builder.append_value(true);
                group_id_builder.append_value(group.id());
                group_name_builder.append_value(group.name());
            } else {
                is_csap_builder.append_value(false);
                group_id_builder.append_null();
                group_name_builder.append_null();
            }
        } else {
            is_csap_builder.append_value(false);
            group_id_builder.append_null();
            group_name_builder.append_null();
        }
    }

    let is_csap_arr: ArrayRef = Arc::new(is_csap_builder.finish());
    let group_id_arr: ArrayRef = Arc::new(group_id_builder.finish());
    let group_name_arr: ArrayRef = Arc::new(group_name_builder.finish());

    let schema = batch.schema();
    let mut new_fields: Vec<Arc<Field>> = Vec::new();
    let mut new_columns: Vec<ArrayRef> = Vec::new();

    let mut has_is_csap = false;
    let mut has_group_id = false;
    let mut has_group_name = false;

    for (idx, field) in schema.fields().iter().enumerate() {
        match field.name().as_str() {
            "is_csap" => {
                new_fields.push(Arc::new(Field::new("is_csap", DataType::Boolean, false)));
                new_columns.push(Arc::clone(&is_csap_arr));
                has_is_csap = true;
            }
            "csap_group_id" => {
                new_fields.push(Arc::new(Field::new("csap_group_id", DataType::UInt8, true)));
                new_columns.push(Arc::clone(&group_id_arr));
                has_group_id = true;
            }
            "csap_group_name" => {
                new_fields.push(Arc::new(Field::new(
                    "csap_group_name",
                    DataType::Utf8,
                    true,
                )));
                new_columns.push(Arc::clone(&group_name_arr));
                has_group_name = true;
            }
            _ => {
                new_fields.push(Arc::clone(field));
                new_columns.push(Arc::clone(batch.column(idx)));
            }
        }
    }

    if !has_is_csap {
        new_fields.push(Arc::new(Field::new("is_csap", DataType::Boolean, false)));
        new_columns.push(is_csap_arr);
    }
    if !has_group_id {
        new_fields.push(Arc::new(Field::new("csap_group_id", DataType::UInt8, true)));
        new_columns.push(group_id_arr);
    }
    if !has_group_name {
        new_fields.push(Arc::new(Field::new(
            "csap_group_name",
            DataType::Utf8,
            true,
        )));
        new_columns.push(group_name_arr);
    }

    let new_schema = Arc::new(Schema::new(new_fields));
    RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| PortError::TransformationError(e.to_string()))
}

/// Calcula o sumário analítico e bioestatístico de CSAP a partir de um lote de internações.
///
/// Suporta esquemas canônicos e DATASUS:
/// - Diagnóstico: `primary_diagnosis`, `main_diagnosis_icd10`, `diag_princ`, `DIAG_PRINC`.
/// - Custo: `total_cost`, `total_paid_amount`, `val_tot`, `VAL_TOT` (Float64/Float32/Int64/UInt64).
/// - Permanência: `length_of_stay`, `length_of_stay_days`, `dias_perm`, `DIAS_PERM` (UInt16/UInt8/UInt32/Int64).
///
/// # Formulação Matemática
///
/// A Taxa Bruta de CSAP por 10.000 habitantes é dada por:
///
/// $$\text{Taxa Bruta CSAP} = \left( \frac{\sum_{i \in \text{CSAP}} N_i}{\text{População}} \right) \times 10.000$$
///
/// O Custo Hospitalar Total Evitável corresponde à soma dos valores totais pagos (`VAL_TOT`):
///
/// $$\text{Custo Evitável} = \sum_{j \in \text{CSAP}} \text{VAL\_TOT}_j$$
pub fn compute_csap_metrics(
    batch: &RecordBatch,
    population: Option<u64>,
) -> Result<CsapMetrics, PortError> {
    let num_rows = batch.num_rows() as u64;
    if num_rows == 0 {
        return Ok(CsapMetrics {
            total_admissions: 0,
            csap_admissions: 0,
            csap_proportion: 0.0,
            csap_rate_per_10k: population.map(|_| 0.0),
            total_cost: 0.0,
            avoidable_cost: 0.0,
            total_days: 0,
            avoidable_days: 0,
            group_counts: [0; 19],
            group_costs: [0.0; 19],
        });
    }

    let diag_col = resolve_diagnosis_column(batch)?;
    let cost_ext = CostExtractor::resolve(batch);
    let days_ext = DaysExtractor::resolve(batch);

    let mut csap_count = 0u64;
    let mut total_cost = 0.0f64;
    let mut avoidable_cost = 0.0f64;
    let mut total_days = 0u64;
    let mut avoidable_days = 0u64;
    let mut group_counts = [0u64; 19];
    let mut group_costs = [0.0f64; 19];

    for i in 0..(num_rows as usize) {
        let cost = cost_ext.get(i);
        let days = days_ext.get(i);

        total_cost += cost;
        total_days += days;

        if diag_col.is_valid(i) {
            let cid = diag_col.value(i);
            if let Some(group) = classify_cid10(cid) {
                csap_count += 1;
                avoidable_cost += cost;
                avoidable_days += days;

                let gid = (group.id() - 1) as usize;
                if gid < 19 {
                    group_counts[gid] += 1;
                    group_costs[gid] += cost;
                }
            }
        }
    }

    let csap_prop = if num_rows > 0 {
        csap_count as f64 / num_rows as f64
    } else {
        0.0
    };

    let rate_per_10k = population.and_then(|pop| {
        if pop > 0 {
            Some((csap_count as f64 / pop as f64) * 10_000.0)
        } else {
            None
        }
    });

    Ok(CsapMetrics {
        total_admissions: num_rows,
        csap_admissions: csap_count,
        csap_proportion: csap_prop,
        csap_rate_per_10k: rate_per_10k,
        total_cost,
        avoidable_cost,
        total_days,
        avoidable_days,
        group_counts,
        group_costs,
    })
}

/// Avalia o Retorno sobre o Investimento (ROI) em Saúde Coletiva na Atenção Primária à Saúde (APS).
///
/// # Formulação Matemática
///
/// Conforme diretrizes bioestatísticas e de economia da saúde da Fiocruz e Ministério da Saúde,
/// o ROI da Atenção Primária sobre internações hospitalares evitáveis é modelado por:
///
/// $$\text{ROI}_{\text{APS}} = \frac{(\alpha \cdot \text{Custo Evitável}) - \text{Investimento}_{\text{APS}}}{\text{Investimento}_{\text{APS}}}$$
///
/// Onde:
/// - $\text{Custo Evitável}$ é o montante financeiro gasto em internações por CSAP ($\sum \text{VAL\_TOT}$).
/// - $\alpha \in [0.0, 1.0]$ é a Fração Atribuível Evitável estimada (tipicamente $0.30$ a $0.50$ na literatura).
/// - $\text{Investimento}_{\text{APS}}$ é o orçamento incremental destinado às equipes de Saúde da Família (eSF/eAP).
///
/// # Exemplos
///
/// ```
/// use brhealth_core::domain::analytics::csap::compute_primary_care_roi;
///
/// // Custo evitável de R$ 1.000.000, fração de impacto 40% (R$ 400.000 de economia real esperada),
/// // com investimento de R$ 200.000 na APS:
/// let roi = compute_primary_care_roi(1_000_000.0, 200_000.0, 0.40).unwrap();
/// assert!((roi - 1.0).abs() < 1e-6); // ROI de 100% (retorno líquido de 1x o investimento)
/// ```
pub fn compute_primary_care_roi(
    avoidable_cost: f64,
    primary_care_investment: f64,
    attributable_fraction: f64,
) -> Result<f64, PortError> {
    if primary_care_investment <= 0.0 {
        return Err(PortError::TransformationError(
            "Investimento na Atenção Primária deve ser estritamente positivo para cálculo de ROI"
                .into(),
        ));
    }
    if !(0.0..=1.0).contains(&attributable_fraction) {
        return Err(PortError::TransformationError(
            "A fração atribuível evitável (alpha) deve estar no intervalo fechado [0.0, 1.0]"
                .into(),
        ));
    }
    if avoidable_cost < 0.0 {
        return Err(PortError::TransformationError(
            "O custo hospitalar evitável não pode ser negativo".into(),
        ));
    }

    let net_savings = (attributable_fraction * avoidable_cost) - primary_care_investment;
    Ok(net_savings / primary_care_investment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{BooleanArray, Float64Array, StringArray, UInt8Array};
    use proptest::prelude::*;

    #[test]
    fn test_all_19_csap_groups_detection() {
        assert_eq!(classify_cid10("A36.0"), Some(CsapGroup::Imunopreveniveis));
        assert_eq!(classify_cid10("A04.9"), Some(CsapGroup::Gastroenterites));
        assert_eq!(classify_cid10("D500"), Some(CsapGroup::Anemia));
        assert_eq!(
            classify_cid10("E41"),
            Some(CsapGroup::DeficienciasNutricionais)
        );
        assert_eq!(
            classify_cid10("J030"),
            Some(CsapGroup::InfeccoesOuvidoNarizGarganta)
        );
        assert_eq!(
            classify_cid10("J14"),
            Some(CsapGroup::PneumoniasBacterianas)
        );
        assert_eq!(classify_cid10("J459"), Some(CsapGroup::Asma));
        assert_eq!(classify_cid10("J440"), Some(CsapGroup::DoencasPulmonares));
        assert_eq!(classify_cid10("I10"), Some(CsapGroup::Hipertensao));
        assert_eq!(classify_cid10("I200"), Some(CsapGroup::Angina));
        assert_eq!(
            classify_cid10("I500"),
            Some(CsapGroup::InsuficienciaCardiaca)
        );
        assert_eq!(
            classify_cid10("I64"),
            Some(CsapGroup::DoencasCerebrovasculares)
        );
        assert_eq!(classify_cid10("E14.2"), Some(CsapGroup::DiabetesMellitus));
        assert_eq!(classify_cid10("G40"), Some(CsapGroup::Epilepsias));
        assert_eq!(
            classify_cid10("N390"),
            Some(CsapGroup::InfeccaoTratoUrinario)
        );
        assert_eq!(classify_cid10("A46"), Some(CsapGroup::InfeccoesPele));
        assert_eq!(
            classify_cid10("N70"),
            Some(CsapGroup::DoencaInflamatoriaPelvica)
        );
        assert_eq!(
            classify_cid10("K250"),
            Some(CsapGroup::UlceraGastrointestinal)
        );
        assert_eq!(classify_cid10("O15"), Some(CsapGroup::DoencasPreNatalParto));
    }

    #[test]
    fn test_non_csap_diagnoses() {
        assert_eq!(classify_cid10("S06"), None); // Traumatismo
        assert_eq!(classify_cid10("C50"), None); // Neoplasia maligna de mama
        assert_eq!(classify_cid10("Z00"), None); // Exame de rotina
        assert!(!is_csap("W01"));
    }

    #[test]
    fn test_enrich_and_metrics_pipeline() {
        // Montar lote sintético com diagnósticos mistos
        let cids = Arc::new(StringArray::from(vec![
            Some("J450"), // CSAP (Asma) - Grupo 7
            Some("S060"), // Não-CSAP (Trauma)
            Some("I10"),  // CSAP (Hipertensão) - Grupo 9
            Some("E119"), // CSAP (Diabetes) - Grupo 13
        ]));
        let costs = Arc::new(Float64Array::from(vec![
            Some(500.0),
            Some(3000.0),
            Some(400.0),
            Some(600.0),
        ]));
        let days = Arc::new(UInt8Array::from(vec![Some(3), Some(10), Some(2), Some(4)]));

        let schema = Arc::new(Schema::new(vec![
            Field::new("primary_diagnosis", DataType::Utf8, true),
            Field::new("total_cost", DataType::Float64, true),
            Field::new("length_of_stay", DataType::UInt8, true),
        ]));

        let batch = RecordBatch::try_new(schema, vec![cids, costs, days]).unwrap();

        // Enriquecimento colunar
        let enriched = enrich_sih_batch_with_csap(&batch).unwrap();
        assert_eq!(enriched.num_columns(), 6);

        let is_csap_col = enriched
            .column_by_name("is_csap")
            .unwrap()
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert!(is_csap_col.value(0));
        assert!(!is_csap_col.value(1));
        assert!(is_csap_col.value(2));
        assert!(is_csap_col.value(3));

        // Métricas bioestatísticas
        let metrics = compute_csap_metrics(&batch, Some(10_000)).unwrap();
        assert_eq!(metrics.total_admissions, 4);
        assert_eq!(metrics.csap_admissions, 3);
        assert!((metrics.csap_proportion - 0.75).abs() < 1e-6);
        let rate = metrics.csap_rate_per_10k.unwrap();
        assert!((rate - 3.0).abs() < 1e-6);
        assert!((metrics.total_cost - 4500.0).abs() < 1e-6);
        assert!((metrics.avoidable_cost - 1500.0).abs() < 1e-6);
        assert_eq!(metrics.total_days, 19);
        assert_eq!(metrics.avoidable_days, 9);

        // Grupo 7 (Asma): índice 6
        assert_eq!(metrics.group_counts[6], 1);
        assert!((metrics.group_costs[6] - 500.0).abs() < 1e-6);
    }

    #[test]
    fn test_csap_with_canonical_sih_schema() {
        use arrow::array::BooleanArray;

        // Montar lote com schema canônico de morbidade hospitalar (SIH):
        // main_diagnosis_icd10, total_paid_amount (Float64), length_of_stay_days (UInt16), is_csap (Boolean)
        let cids = Arc::new(StringArray::from(vec![
            Some("J450"), // CSAP (Asma) - R$ 1200, 4 dias
            Some("S060"), // Não-CSAP (Trauma) - R$ 2500, 8 dias
            Some("I10"),  // CSAP (Hipertensão) - R$ 800, 2 dias
        ]));
        let costs = Arc::new(Float64Array::from(vec![
            Some(1200.0),
            Some(2500.0),
            Some(800.0),
        ]));
        let days = Arc::new(UInt16Array::from(vec![Some(4), Some(8), Some(2)]));
        let initial_is_csap = Arc::new(BooleanArray::from(vec![false, false, false]));

        let schema = Arc::new(Schema::new(vec![
            Field::new("main_diagnosis_icd10", DataType::Utf8, false),
            Field::new("total_paid_amount", DataType::Float64, false),
            Field::new("length_of_stay_days", DataType::UInt16, false),
            Field::new("is_csap", DataType::Boolean, false),
        ]));

        let batch = RecordBatch::try_new(schema, vec![cids, costs, days, initial_is_csap]).unwrap();

        // Enriquecer e verificar substituição idempotente de is_csap
        let enriched = enrich_sih_batch_with_csap(&batch).unwrap();
        // Não deve duplicar is_csap: 4 originais + 2 novos (csap_group_id, csap_group_name) = 6
        assert_eq!(enriched.num_columns(), 6);

        let is_csap_col = enriched
            .column_by_name("is_csap")
            .unwrap()
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert!(is_csap_col.value(0)); // J450 é CSAP
        assert!(!is_csap_col.value(1)); // S060 não é CSAP
        assert!(is_csap_col.value(2)); // I10 é CSAP

        // Computar métricas utilizando os nomes canônicos e tipos nativos
        let metrics = compute_csap_metrics(&batch, Some(50_000)).unwrap();
        assert_eq!(metrics.total_admissions, 3);
        assert_eq!(metrics.csap_admissions, 2);
        assert!((metrics.total_cost - 4500.0).abs() < 1e-6);
        assert!((metrics.avoidable_cost - 2000.0).abs() < 1e-6);
        assert_eq!(metrics.total_days, 14);
        assert_eq!(metrics.avoidable_days, 6);
    }

    #[test]
    fn test_roi_calculations() {
        // Custo evitável: R$ 500.000
        // Fração de impacto: 50% -> Economia de R$ 250.000
        // Investimento: R$ 100.000
        // Retorno Líquido: R$ 150.000 / R$ 100.000 = 1.5 (150% de ROI)
        let roi = compute_primary_care_roi(500_000.0, 100_000.0, 0.50).unwrap();
        assert!((roi - 1.5).abs() < 1e-6);

        // Erro: Investimento não positivo
        assert!(compute_primary_care_roi(500_000.0, 0.0, 0.5).is_err());
        // Erro: Fração fora do intervalo [0, 1]
        assert!(compute_primary_care_roi(500_000.0, 100_000.0, 1.2).is_err());
    }

    proptest! {
        #[test]
        fn prop_test_never_panics_on_arbitrary_cids(s in "\\PC*") {
            let _ = classify_cid10(&s);
            let _ = is_csap(&s);
        }

        #[test]
        fn prop_test_roi_is_linear_with_avoidable_cost(
            cost in 0.0f64..1_000_000.0f64,
            investment in 1.0f64..100_000.0f64,
            alpha in 0.0f64..1.0f64
        ) {
            let res = compute_primary_care_roi(cost, investment, alpha);
            prop_assert!(res.is_ok());
            let roi = res.unwrap();
            let expected = ((alpha * cost) - investment) / investment;
            prop_assert!((roi - expected).abs() < 1e-5);
        }
    }
}
