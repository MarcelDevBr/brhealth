// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Bindings idiomáticos de alta performance para Python via PyO3.
//!
//! Permite consumo de pipelines analíticos, harmonização territorial do IBGE,
//! classificação de CSAP e exportação Zero-Copy de `RecordBatch` para Polars e PyArrow.

#![allow(clippy::useless_conversion)]

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Arc;

use arrow::array::{Array, ArrayData, StructArray};
use arrow::ffi::to_ffi;
use arrow::record_batch::RecordBatch;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyCapsule, PyDict};

use brhealth_core::domain::analytics::csap::{
    classify_cid10 as core_classify_cid10, compute_csap_metrics, compute_primary_care_roi,
    is_csap as core_is_csap,
};
use brhealth_core::domain::application::{
    BRHealthApplicationService, PipelineExecutionOptions,
};
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, SourceExecutionContext,
};
use brhealth_core::domain::spatial::h3::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::{
    calculate_ibge_dv as core_calculate_ibge_dv, harmonize_ibge_code as core_harmonize_ibge_code,
};
use brhealth_core::domain::transforms::ontology::MedicalOntologyHarmonizer;
use brhealth_core::infrastructure::cache::MemoryCache;
use brhealth_core::infrastructure::state::MemorySyncState;
use brhealth_core::infrastructure::transport::{AsyncFtpTransport, AsyncHttpTransport};
use brhealth_core::sources::{create_pack_brasil, create_pack_global};
use brhealth_core::{FairManifest, SourceRegistry};

/// Wrapper colunar para RecordBatch com exportação Arrow Zero-Copy (PyCapsule / C Data Interface).
#[pyclass]
pub struct RecordBatchWrapper {
    batch: RecordBatch,
    manifest: Option<FairManifest>,
}

impl RecordBatchWrapper {
    /// Cria uma nova instância a partir de um RecordBatch.
    pub fn new(batch: RecordBatch, manifest: Option<FairManifest>) -> Self {
        Self { batch, manifest }
    }
}

#[pymethods]
impl RecordBatchWrapper {
    /// Quantidade de linhas contidas no lote colunar.
    #[getter]
    pub fn num_rows(&self) -> usize {
        self.batch.num_rows()
    }

    /// Quantidade de colunas contidas no lote colunar.
    #[getter]
    pub fn num_columns(&self) -> usize {
        self.batch.num_columns()
    }

    /// Nomes de todos os campos/colunas do lote.
    pub fn column_names(&self) -> Vec<String> {
        self.batch
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect()
    }

    /// Retorna os endereços de memória brutos (array_ptr, schema_ptr) para a C Data Interface.
    pub fn to_arrow_pointers(&self) -> PyResult<(usize, usize)> {
        let struct_array: StructArray = self.batch.clone().into();
        let data: ArrayData = struct_array.to_data();

        let (ffi_array, ffi_schema) = to_ffi(&data)
            .map_err(|e| PyValueError::new_err(format!("Falha na conversão FFI: {e}")))?;

        let boxed_array = Box::into_raw(Box::new(ffi_array));
        let boxed_schema = Box::into_raw(Box::new(ffi_schema));

        Ok((boxed_array as usize, boxed_schema as usize))
    }

    /// Protocolo Arrow PyCapsule: Retorna o schema do lote em um PyCapsule ('arrow_schema').
    pub fn __arrow_c_schema__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyCapsule>> {
        let struct_array: StructArray = self.batch.clone().into();
        let data: ArrayData = struct_array.to_data();
        let (_, ffi_schema) = to_ffi(&data)
            .map_err(|e| PyValueError::new_err(format!("Falha ao gerar schema FFI: {e}")))?;

        let name = CString::new("arrow_schema").map_err(|e| PyValueError::new_err(e.to_string()))?;
        PyCapsule::new_bound(py, ffi_schema, Some(name))
    }

    /// Protocolo Arrow PyCapsule: Retorna a tupla (schema, array) em PyCapsules para consumo Zero-Copy.
    #[allow(deprecated)]
    #[pyo3(signature = (_requested_schema=None))]
    pub fn __arrow_c_array__<'py>(
        &self,
        py: Python<'py>,
        _requested_schema: Option<PyObject>,
    ) -> PyResult<(Bound<'py, PyCapsule>, Bound<'py, PyCapsule>)> {
        let struct_array: StructArray = self.batch.clone().into();
        let data: ArrayData = struct_array.to_data();
        let (ffi_array, ffi_schema) = to_ffi(&data)
            .map_err(|e| PyValueError::new_err(format!("Falha ao gerar array FFI: {e}")))?;

        let schema_name =
            CString::new("arrow_schema").map_err(|e| PyValueError::new_err(e.to_string()))?;
        let array_name =
            CString::new("arrow_array").map_err(|e| PyValueError::new_err(e.to_string()))?;

        let schema_capsule = PyCapsule::new_bound(py, ffi_schema, Some(schema_name))?;
        let array_capsule = PyCapsule::new_bound(py, ffi_array, Some(array_name))?;

        Ok((schema_capsule, array_capsule))
    }

    /// Converte diretamente o RecordBatch para um objeto PyArrow RecordBatch via Arrow PyCapsule.
    pub fn to_pyarrow<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let pyarrow = py.import_bound("pyarrow")?;
        pyarrow.call_method1("record_batch", (slf.into_py(py),))
    }

    /// Converte diretamente o RecordBatch para um DataFrame Polars via Arrow PyCapsule.
    pub fn to_polars<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let polars = py.import_bound("polars")?;
        polars.call_method1("from_arrow", (slf.into_py(py),))
    }

    /// Exporta o manifesto FAIR W3C PROV-O em formato JSON-LD para o caminho de arquivo fornecido.
    pub fn export_fair_manifest(&self, path: &str) -> PyResult<()> {
        if let Some(ref manifest) = self.manifest {
            let json = manifest
                .to_json()
                .map_err(|e| PyValueError::new_err(format!("Erro ao gerar JSON FAIR: {e}")))?;
            std::fs::write(path, json)
                .map_err(|e| PyValueError::new_err(format!("Erro ao gravar arquivo FAIR: {e}")))?;
            Ok(())
        } else {
            Err(PyValueError::new_err(
                "Nenhum manifesto FAIR disponível neste lote",
            ))
        }
    }
}

/// Calcula o Dígito Verificador oficial do IBGE (Módulo 10 Luhn) para um código de 6 dígitos.
#[pyfunction]
pub fn calculate_ibge_dv(code_6_digits: &str) -> PyResult<u8> {
    core_calculate_ibge_dv(code_6_digits).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Harmoniza um código municipal (de 6 ou 7 dígitos) para a representação canônica de 7 dígitos.
#[pyfunction]
pub fn harmonize_ibge_code(raw_code: &str) -> PyResult<String> {
    core_harmonize_ibge_code(raw_code).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte coordenadas de latitude e longitude em um índice hexagonal Uber H3.
#[pyfunction]
pub fn latlng_to_h3(lat: f64, lng: f64, resolution: u8) -> PyResult<u64> {
    coord_to_h3_index(lat, lng, resolution).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Classifica um código de diagnóstico CID-10 conforme os 19 grupos da Portaria MS/SAS nº 221/2008.
#[pyfunction]
pub fn classify_cid10(cid: &str) -> Option<u8> {
    core_classify_cid10(cid).map(|g| g.id())
}

/// Verifica se um código CID-10 pertence à Lista Brasileira de CSAP (Portaria 221/2008).
#[pyfunction]
pub fn is_csap(cid: &str) -> bool {
    core_is_csap(cid)
}

/// Mapeia um código de mortalidade ou morbidade da CID-10 para a CID-11.
#[pyfunction]
pub fn map_icd10_to_icd11(icd10: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd10_to_icd11(icd10)
}

/// Mapeia um código da CID-10 para o código de conceito SNOMED-CT (SCTID).
#[pyfunction]
pub fn map_icd10_to_snomed(icd10: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd10_to_snomed(icd10)
}

/// Calcula o Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária.
#[pyfunction]
pub fn compute_roi(
    avoidable_cost: f64,
    investment: f64,
    attributable_fraction: f64,
) -> PyResult<f64> {
    compute_primary_care_roi(avoidable_cost, investment, attributable_fraction)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}


/// Motor de computação e registro analítico do BRHealth.
#[pyclass]
pub struct Engine {
    app_service: Arc<BRHealthApplicationService>,
    rt: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl Engine {
    /// Inicializa o motor com todos os pacotes oficiais (Brasil e Global) registrados.
    #[new]
    pub fn new() -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Falha ao iniciar runtime Tokio: {e}")))?;

        let mut reg = SourceRegistry::new();
        reg.register_pack(create_pack_brasil());
        reg.register_pack(create_pack_global());
        let registry = Arc::new(reg);

        let ftp_transport = Arc::new(AsyncFtpTransport::new_datasus());
        let _http_transport = match AsyncHttpTransport::new_default() {
            Ok(t) => Arc::new(t),
            Err(_) => Arc::new(AsyncHttpTransport::new_default().map_err(|e| {
                PyValueError::new_err(format!("Falha ao inicializar HTTP: {e}"))
            })?),
        };

        let decompressor = match brhealth_core::decoders::dbc::DbcDecompressor::new() {
            Ok(d) => Arc::new(d),
            Err(e) => return Err(PyValueError::new_err(e.to_string())),
        };

        let sync_state = Arc::new(MemorySyncState::new());
        let context = Arc::new(SourceExecutionContext {
            transport: ftp_transport,
            decompressor,
            cache: Arc::new(MemoryCache::new()),
            state: sync_state.clone(),
        });

        let app_service =
            Arc::new(BRHealthApplicationService::new(registry, context, sync_state));

        Ok(Self {
            app_service,
            rt: Arc::new(rt),
        })
    }

    /// Retorna a lista de identificadores das fontes registradas.
    pub fn list_sources(&self) -> Vec<String> {
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

    /// Executa ingestão e harmonização de uma fonte de dados de saúde.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source_id, jurisdiction=None, year=2024, month=None, harmonize_ibge=true, assign_h3=None, enrich_csap=false))]
    pub fn fetch(
        &self,
        source_id: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
        assign_h3: Option<u8>,
        enrich_csap: bool,
    ) -> PyResult<RecordBatchWrapper> {
        let params = DataQueryParams {
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            jurisdiction_code: jurisdiction,
            year,
            month,
            extra_filters: HashMap::new(),
            as_of_snapshot: None,
        };

        let options = PipelineExecutionOptions {
            harmonize_ibge,
            assign_h3_resolution: assign_h3,
            h3_coord_columns: None,
            enrich_csap,
            reference_population: None,
            persist_to_cache: false,
            cache_base_path: None,
        };

        let app = self.app_service.clone();
        let source_id_str = source_id.to_string();

        let result = self
            .rt
            .block_on(async move {
                app.execute_full_pipeline(&source_id_str, &params, &options)
                    .await
            })
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let combined_batch = if result.batches.is_empty() {
            let src = self
                .app_service
                .registry()
                .get(source_id)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            RecordBatch::new_empty(src.target_schema())
        } else {
            result.batches[0].clone()
        };

        Ok(RecordBatchWrapper::new(combined_batch, Some(result.manifest)))
    }

    /// Avalia um RecordBatch de internações hospitalares e calcula métricas de CSAP.
    #[pyo3(signature = (wrapper, reference_population=None))]
    pub fn evaluate_csap<'py>(
        &self,
        py: Python<'py>,
        wrapper: &RecordBatchWrapper,
        reference_population: Option<u64>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let metrics = compute_csap_metrics(&wrapper.batch, reference_population)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let avoidable_cost_proportion = if metrics.total_cost > 0.0 {
            (metrics.avoidable_cost / metrics.total_cost) * 100.0
        } else {
            0.0
        };

        let dict = PyDict::new_bound(py);
        dict.set_item("total_admissions", metrics.total_admissions)?;
        dict.set_item("csap_admissions", metrics.csap_admissions)?;
        dict.set_item("csap_proportion", metrics.csap_proportion)?;
        dict.set_item("total_cost", metrics.total_cost)?;
        dict.set_item("avoidable_cost", metrics.avoidable_cost)?;
        dict.set_item("avoidable_cost_proportion", avoidable_cost_proportion)?;
        dict.set_item("avoidable_days", metrics.avoidable_days)?;
        dict.set_item("csap_rate_per_10k", metrics.csap_rate_per_10k)?;

        Ok(dict)
    }

    /// Valida a consistência biológica de um evento médico de acordo com sexo e idade.
    pub fn validate_biological_consistency(
        &self,
        icd10: &str,
        sex: &str,
        age_years: u16,
    ) -> PyResult<bool> {
        let harmonizer = MedicalOntologyHarmonizer::new();
        let bio_sex =
            brhealth_core::domain::transforms::ontology::BiologicalSex::from_str_lenient(sex);
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
        Self::new().expect("Falha ao inicializar Engine com configuração padrão")
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
    m.add_function(wrap_pyfunction!(compute_roi, m)?)?;
    m.add_class::<Engine>()?;
    m.add_class::<RecordBatchWrapper>()?;

    Ok(())
}
