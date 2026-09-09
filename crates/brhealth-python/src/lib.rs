// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Bindings idiomáticos de alta performance para Python via PyO3.
//!
//! Permite consumo de pipelines analíticos, harmonização territorial do IBGE,
//! classificação de CSAP e exportação Zero-Copy de `RecordBatch` para Polars e PyArrow.

#![allow(clippy::useless_conversion)]

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use brhealth_core::domain::analytics::csap::{
    classify_cid10 as core_classify_cid10, is_csap as core_is_csap,
};
use brhealth_core::domain::spatial::h3::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::{
    calculate_ibge_dv as core_calculate_ibge_dv, harmonize_ibge_code as core_harmonize_ibge_code,
};
use brhealth_core::domain::transforms::ontology::MedicalOntologyHarmonizer;
use brhealth_core::sources::{create_pack_brasil, create_pack_global};
use brhealth_core::SourceRegistry;

/// Calcula o Dígito Verificador oficial do IBGE (Módulo 10 Luhn) para um código de 6 dígitos.
#[pyfunction]
fn calculate_ibge_dv(code_6_digits: &str) -> PyResult<u8> {
    core_calculate_ibge_dv(code_6_digits).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Harmoniza um código municipal (de 6 ou 7 dígitos) para a representação canônica de 7 dígitos.
#[pyfunction]
fn harmonize_ibge_code(raw_code: &str) -> PyResult<String> {
    core_harmonize_ibge_code(raw_code).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte coordenadas de latitude e longitude em um índice hexagonal Uber H3.
#[pyfunction]
fn latlng_to_h3(lat: f64, lng: f64, resolution: u8) -> PyResult<u64> {
    coord_to_h3_index(lat, lng, resolution).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Classifica um código de diagnóstico CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008.
#[pyfunction]
fn classify_cid10(cid: &str) -> Option<u8> {
    core_classify_cid10(cid).map(|g| g.id())
}

/// Verifica se um código CID-10 pertence à Lista Brasileira de CSAP (Portaria 221/2008).
#[pyfunction]
fn is_csap(cid: &str) -> bool {
    core_is_csap(cid)
}

/// Mapeia um código de mortalidade ou morbidade da CID-10 para a CID-11.
#[pyfunction]
fn map_icd10_to_icd11(icd10: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd10_to_icd11(icd10)
}

/// Mapeia um código da CID-10 para o código de conceito SNOMED-CT (SCTID).
#[pyfunction]
fn map_icd10_to_snomed(icd10: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd10_to_snomed(icd10)
}

/// Motor de computação e registro analítico do BRHealth.
#[pyclass]
pub struct Engine {
    registry: Arc<SourceRegistry>,
}

#[pymethods]
impl Engine {
    /// Inicializa o motor com todos os pacotes oficiais (Brasil e Global) registrados.
    #[new]
    pub fn new() -> Self {
        let mut reg = SourceRegistry::new();
        reg.register_pack(create_pack_brasil());
        reg.register_pack(create_pack_global());

        Self {
            registry: Arc::new(reg),
        }
    }

    /// Retorna a lista de identificadores das fontes registradas.
    pub fn list_sources(&self) -> Vec<String> {
        let _ = &self.registry;
        let br_pack = create_pack_brasil();
        let global_pack = create_pack_global();

        let mut ids = Vec::with_capacity(br_pack.len() + global_pack.len());
        for s in br_pack {
            ids.push(s.metadata().id.to_string());
        }
        for s in global_pack {
            ids.push(s.metadata().id.to_string());
        }
        ids
    }

    /// Retorna o número total de fontes registradas.
    pub fn source_count(&self) -> usize {
        let br = create_pack_brasil().len();
        let global = create_pack_global().len();
        br + global
    }

    /// Valida a consistência biológica de um evento médico de acordo com sexo e idade.
    pub fn validate_biological_consistency(
        &self,
        icd10: &str,
        sex: &str,
        age_years: u16,
    ) -> PyResult<bool> {
        let harmonizer = MedicalOntologyHarmonizer::new();
        let bio_sex = brhealth_core::domain::transforms::ontology::BiologicalSex::from_str_lenient(sex);
        match harmonizer.validate_biological_consistency(icd10, bio_sex, age_years) {
            Ok(()) => Ok(true),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    /// Retorna a versão do motor analítico.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Módulo Python do BRHealth.
#[pymodule]
fn brhealth(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(calculate_ibge_dv, m)?)?;
    m.add_function(wrap_pyfunction!(harmonize_ibge_code, m)?)?;
    m.add_function(wrap_pyfunction!(latlng_to_h3, m)?)?;
    m.add_function(wrap_pyfunction!(classify_cid10, m)?)?;
    m.add_function(wrap_pyfunction!(is_csap, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd10_to_icd11, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd10_to_snomed, m)?)?;
    m.add_class::<Engine>()?;

    Ok(())
}
