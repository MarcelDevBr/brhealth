// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Bindings idiomáticos de alta performance para Python via PyO3.
//!
//! Permite consumo de pipelines analíticos, descompressão nativa PKWARE Blast (.dbc)
//! e decodificação DBF para Apache Arrow, harmonização territorial do IBGE,
//! classificação de CSAP, indexação espacial discreta H3/S2, análise de mortalidade APVP,
//! e exportação Zero-Copy de `RecordBatch` para Polars, PyArrow, Pandas e PyTorch via DLPack.

#![allow(clippy::useless_conversion)]
#![allow(unexpected_cfgs)]

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, OnceLock};

use arrow::array::{Array, ArrayData, StructArray};
use arrow::compute::concat_batches;
use arrow::ffi::to_ffi;
use arrow::record_batch::RecordBatch;
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyCapsule, PyDict};

create_exception!(brhealth, BRHealthError, pyo3::exceptions::PyException);
create_exception!(brhealth, SourceNotFoundError, BRHealthError);
create_exception!(brhealth, TransportError, BRHealthError);
create_exception!(brhealth, ValidationError, BRHealthError);

pub fn safe_catch_panic<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce() -> PyResult<T> + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(res) => res,
        Err(cause) => {
            let msg = if let Some(s) = cause.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = cause.downcast_ref::<String>() {
                s.clone()
            } else {
                "Panic interno não identificado no motor Rust".to_string()
            };
            Err(BRHealthError::new_err(format!(
                "Erro crítico no motor BRHealth: {msg}"
            )))
        }
    }
}

use brhealth_core::decoders::{DbcDecompressor, DbfDecoder};
use brhealth_core::domain::analytics::csap::{
    classify_cid10 as core_classify_cid10, compute_csap_metrics, compute_primary_care_roi,
    is_csap as core_is_csap, CsapGroup,
};
use brhealth_core::domain::analytics::mortality::{
    compute_age_standardized_mortality_rate as core_compute_age_standardized_mortality_rate,
    compute_apvp as core_compute_apvp, compute_apvp_rate as core_compute_apvp_rate,
    compute_batch_apvp as core_compute_batch_apvp,
};
use brhealth_core::domain::application::{BRHealthApplicationService, PipelineExecutionOptions};
use brhealth_core::domain::source_spi::{DataQueryParams, GeographicScope};
use brhealth_core::domain::spatial::h3::{
    coord_to_h3_index, h3_grid_disk as core_h3_grid_disk,
    h3_grid_distance as core_h3_grid_distance, h3_index_to_coord as core_h3_index_to_coord,
};
use brhealth_core::domain::spatial::s2::{
    coord_to_s2_cell as core_coord_to_s2_cell, s2_cell_to_coord as core_s2_cell_to_coord,
    DEFAULT_S2_MUNICIPAL_LEVEL,
};
use brhealth_core::domain::transforms::ibge::{
    calculate_ibge_dv as core_calculate_ibge_dv, harmonize_ibge_code as core_harmonize_ibge_code,
    reconcile_historical_ibge_code as core_reconcile_historical_ibge_code,
    validate_ibge_code as core_validate_ibge_code,
};
use brhealth_core::domain::transforms::ontology::{
    BiologicalSex, Icd10Chapter, MedicalOntologyHarmonizer,
};
use brhealth_core::domain::transforms::pharmacy::PharmacyHarmonizer;
use brhealth_core::domain::transforms::sigtap::{
    is_amputation_procedure as core_is_amputation_procedure,
    is_dialysis_procedure as core_is_dialysis_procedure,
    parse_sigtap_code as core_parse_sigtap_code,
};
use brhealth_core::FairManifest;

/// Wrapper colunar para RecordBatch com exportação Arrow Zero-Copy (PyCapsule / C Data Interface / DLPack).
#[pyclass]
#[derive(Clone)]
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

    /// Dimensões do lote colunar (linhas, colunas).
    #[getter]
    pub fn shape(&self) -> (usize, usize) {
        (self.batch.num_rows(), self.batch.num_columns())
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

    /// Lista de colunas do lote (propriedade compatível com Pandas/Polars).
    #[getter]
    pub fn columns(&self) -> Vec<String> {
        self.column_names()
    }

    /// Dicionário contendo o esquema de tipos Arrow {nome_coluna: tipo_string}.
    #[getter]
    pub fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);
        for field in self.batch.schema().fields() {
            dict.set_item(field.name(), format!("{:?}", field.data_type()))?;
        }
        Ok(dict)
    }

    /// Retorna o manifesto FAIR W3C PROV-O formatado em JSON, se disponível.
    #[getter]
    pub fn manifest_json(&self) -> Option<String> {
        self.manifest.as_ref().and_then(|m| m.to_json().ok())
    }

    /// Suporte ao protocolo len() do Python: retorna o número de linhas contidas no lote.
    pub fn __len__(&self) -> usize {
        self.batch.num_rows()
    }

    /// Representação textual concisa do lote colunar.
    pub fn __repr__(&self) -> String {
        format!(
            "BRHealth.RecordBatch({} rows x {} columns, fields={:?})",
            self.batch.num_rows(),
            self.batch.num_columns(),
            self.column_names()
        )
    }

    /// Retorna um novo RecordBatchWrapper contendo as primeiras `n` linhas (padrão 5) com Zero-Copy.
    #[pyo3(signature = (n=None))]
    pub fn head(&self, n: Option<usize>) -> Self {
        let limit = n.unwrap_or(5).min(self.batch.num_rows());
        let sliced = self.batch.slice(0, limit);
        RecordBatchWrapper::new(sliced, self.manifest.clone())
    }

    /// Retorna um novo RecordBatchWrapper contendo as últimas `n` linhas (padrão 5) com Zero-Copy.
    #[pyo3(signature = (n=None))]
    pub fn tail(&self, n: Option<usize>) -> Self {
        let n = n.unwrap_or(5);
        let total = self.batch.num_rows();
        let offset = total.saturating_sub(n);
        let limit = total - offset;
        let sliced = self.batch.slice(offset, limit);
        RecordBatchWrapper::new(sliced, self.manifest.clone())
    }

    /// Renderização rica em HTML para exibição interativa e elegante no Jupyter Notebook e Google Colab.
    pub fn _repr_html_(&self) -> String {
        let schema = self.batch.schema();
        let mut fields_html = String::new();
        for field in schema.fields() {
            fields_html.push_str(&format!(
                "<tr><td style='text-align:left;font-weight:600;padding:4px 12px;border-bottom:1px solid #e0e0e0;'>{}</td><td style='text-align:left;color:#555;padding:4px 12px;border-bottom:1px solid #e0e0e0;'>{:?}</td><td style='text-align:center;color:#888;padding:4px 12px;border-bottom:1px solid #e0e0e0;'>{}</td></tr>",
                field.name(),
                field.data_type(),
                if field.is_nullable() { "sim" } else { "não" }
            ));
        }

        format!(
            "<div style='border:1px solid #0284c7;border-radius:8px;padding:12px;background:#f8fafc;font-family:system-ui,sans-serif;max-width:650px;'>\
            <div style='display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;'>\
            <span style='font-size:14px;font-weight:bold;color:#0369a1;'>BRHealth Colunar RecordBatch (Apache Arrow Zero-Copy)</span>\
            <span style='background:#0284c7;color:white;font-size:11px;padding:2px 8px;border-radius:12px;'>{} linhas &times; {} colunas</span>\
            </div>\
            <table style='width:100%;border-collapse:collapse;font-size:12px;'>\
            <thead><tr style='background:#e2e8f0;'><th style='text-align:left;padding:6px 12px;'>Coluna</th><th style='text-align:left;padding:6px 12px;'>Tipo Arrow</th><th style='text-align:center;padding:6px 12px;'>Anulável</th></tr></thead>\
            <tbody>{}</tbody>\
            </table>\
            </div>",
            self.batch.num_rows(),
            self.batch.num_columns(),
            fields_html
        )
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

        let name =
            CString::new("arrow_schema").map_err(|e| PyValueError::new_err(e.to_string()))?;
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

    /// Retorna o dispositivo DLPack (1 = kDLCPU, 0 = device_id).
    pub fn __dlpack_device__(&self) -> (i32, i32) {
        (1, 0)
    }

    /// Implementação do protocolo DLPack para interoperabilidade com PyTorch e tensores sem cópia.
    #[pyo3(signature = (stream=None, _max_version=None, _dl_device=None, _copy=None))]
    pub fn __dlpack__<'py>(
        slf: PyRef<'py, Self>,
        py: Python<'py>,
        stream: Option<PyObject>,
        _max_version: Option<PyObject>,
        _dl_device: Option<PyObject>,
        _copy: Option<PyObject>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let slf_py: Py<Self> = slf.into();
        if let Ok(pa) = py.import_bound("pyarrow") {
            let batch = pa.call_method1("record_batch", (slf_py.clone_ref(py),))?;
            let stream_arg = stream.as_ref().map(|s| s.clone_ref(py));
            if let Ok(res) = batch.call_method1("__dlpack__", (stream_arg,)) {
                return Ok(res);
            }
        }
        if let Ok(pl) = py.import_bound("polars") {
            let df = pl.call_method1("from_arrow", (slf_py.clone_ref(py),))?;
            if let Ok(res) = df.call_method1("__dlpack__", (stream,)) {
                return Ok(res);
            }
        }
        Err(PyValueError::new_err(
            "DLPack requer 'pyarrow' ou 'polars' instalado no ambiente Python",
        ))
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

    /// Converte o RecordBatch para um DataFrame do Pandas.
    pub fn to_pandas<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let slf_py: Py<Self> = slf.into();
        if let Ok(pa) = py.import_bound("pyarrow") {
            let batch = pa.call_method1("record_batch", (slf_py.clone_ref(py),))?;
            if let Ok(df) = batch.call_method0("to_pandas") {
                return Ok(df);
            }
        }
        if let Ok(pl) = py.import_bound("polars") {
            let df = pl.call_method1("from_arrow", (slf_py.clone_ref(py),))?;
            if let Ok(pandas_df) = df.call_method0("to_pandas") {
                return Ok(pandas_df);
            }
        }
        Err(PyValueError::new_err(
            "to_pandas() requer 'pyarrow' ou 'polars' e 'pandas' instalados no ambiente Python",
        ))
    }

    /// Converte o RecordBatch para um dicionário Python {coluna: lista_valores}.
    pub fn to_dict<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let slf_py: Py<Self> = slf.into();
        if let Ok(pa) = py.import_bound("pyarrow") {
            let batch = pa.call_method1("record_batch", (slf_py.clone_ref(py),))?;
            if let Ok(dict) = batch.call_method0("to_pydict") {
                return Ok(dict);
            }
        }
        if let Ok(pl) = py.import_bound("polars") {
            let df = pl.call_method1("from_arrow", (slf_py.clone_ref(py),))?;
            if let Ok(dict) = df.call_method0("to_dict") {
                return Ok(dict);
            }
        }
        Err(PyValueError::new_err(
            "to_dict() requer 'pyarrow' ou 'polars' instalado no ambiente Python",
        ))
    }

    /// Alias para `to_dict()`, compatível com o método padrão do PyArrow RecordBatch.
    pub fn to_pydict<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Self::to_dict(slf, py)
    }

    /// Suporte ao operador colchetes (`batch["coluna"]` ou `batch[0]`): delega para PyArrow ou Polars com Zero-Copy.
    pub fn __getitem__<'py>(
        slf: PyRef<'py, Self>,
        py: Python<'py>,
        key: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let slf_py: Py<Self> = slf.into();
        if let Ok(pa) = py.import_bound("pyarrow") {
            let batch = pa.call_method1("record_batch", (slf_py.clone_ref(py),))?;
            return batch.call_method1("__getitem__", (key,));
        }
        if let Ok(pl) = py.import_bound("polars") {
            let df = pl.call_method1("from_arrow", (slf_py.clone_ref(py),))?;
            return df.call_method1("__getitem__", (key,));
        }
        Err(PyValueError::new_err(
            "Indexação via __getitem__ requer 'pyarrow' ou 'polars' instalado no ambiente Python",
        ))
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

// ---------------------------------------------------------------------------
// Decodificadores Nativos (.dbc e .dbf)
// ---------------------------------------------------------------------------

/// Descomprime um arquivo .dbc do DATASUS e decodifica diretamente para um RecordBatch Arrow Zero-Copy.
///
/// Libera o GIL do Python durante o I/O e a descompressão Blast para máxima responsividade.
#[pyfunction]
pub fn read_dbc(py: Python<'_>, path: &str) -> PyResult<RecordBatchWrapper> {
    let path_str = path.to_string();
    let batch = py
        .allow_threads(|| -> Result<RecordBatch, String> {
            let input = std::fs::read(&path_str)
                .map_err(|e| format!("Erro ao ler arquivo '{path_str}': {e}"))?;
            let decompressor = DbcDecompressor::new()
                .map_err(|e| format!("Falha ao inicializar descompressor DBC: {e}"))?;
            let dbf_bytes = decompressor
                .decompress_dbc(&input)
                .map_err(|e| format!("Falha na descompressão Blast do DBC: {e}"))?;
            let decoder = DbfDecoder::new();
            decoder
                .decode_to_record_batch(&dbf_bytes)
                .map_err(|e| format!("Falha na decodificação DBF: {e}"))
        })
        .map_err(BRHealthError::new_err)?;
    Ok(RecordBatchWrapper::new(batch, None))
}

/// Decodifica um arquivo .dbf diretamente para um RecordBatch Arrow Zero-Copy.
///
/// Libera o GIL do Python durante o I/O e a decodificação DBF.
#[pyfunction]
pub fn read_dbf(py: Python<'_>, path: &str) -> PyResult<RecordBatchWrapper> {
    let path_str = path.to_string();
    let batch = py
        .allow_threads(|| -> Result<RecordBatch, String> {
            let input = std::fs::read(&path_str)
                .map_err(|e| format!("Erro ao ler arquivo '{path_str}': {e}"))?;
            let decoder = DbfDecoder::new();
            decoder
                .decode_to_record_batch(&input)
                .map_err(|e| format!("Falha na decodificação DBF: {e}"))
        })
        .map_err(BRHealthError::new_err)?;
    Ok(RecordBatchWrapper::new(batch, None))
}

/// Descomprime um arquivo .dbc do DATASUS retornando os bytes brutos do arquivo .dbf correspondente.
///
/// Caso `output_path` seja fornecido, grava os bytes descomprimidos no caminho de arquivo especificado.
/// Libera o GIL do Python durante o processamento.
#[pyfunction]
#[pyo3(signature = (input_path, output_path=None))]
pub fn decompress_dbc<'py>(
    py: Python<'py>,
    input_path: &str,
    output_path: Option<String>,
) -> PyResult<Bound<'py, PyBytes>> {
    let input_path_str = input_path.to_string();

    let dbf_bytes = py
        .allow_threads(move || -> Result<Vec<u8>, String> {
            let input = std::fs::read(&input_path_str)
                .map_err(|e| format!("Erro ao ler arquivo '{input_path_str}': {e}"))?;
            let decompressor = DbcDecompressor::new()
                .map_err(|e| format!("Falha ao inicializar descompressor DBC: {e}"))?;
            let dbf_bytes = decompressor
                .decompress_dbc(&input)
                .map_err(|e| format!("Falha na descompressão Blast do DBC: {e}"))?;

            if let Some(ref out_path) = output_path {
                std::fs::write(out_path, &dbf_bytes)
                    .map_err(|e| format!("Erro ao gravar DBF em '{out_path}': {e}"))?;
            }
            Ok(dbf_bytes)
        })
        .map_err(BRHealthError::new_err)?;

    Ok(PyBytes::new_bound(py, &dbf_bytes))
}

// ---------------------------------------------------------------------------
// Harmonização Territorial IBGE
// ---------------------------------------------------------------------------

/// Calcula o Dígito Verificador oficial do IBGE (Módulo 10 Luhn) para um código de 6 dígitos.
///
/// Aceita string ou inteiro (ex: `"355030"` ou `355030`).
#[pyfunction]
pub fn calculate_ibge_dv(code_6_digits: &Bound<'_, PyAny>) -> PyResult<u8> {
    let s = if let Ok(s) = code_6_digits.extract::<String>() {
        s
    } else if let Ok(n) = code_6_digits.extract::<i64>() {
        n.to_string()
    } else {
        return Err(PyValueError::new_err(
            "Código IBGE deve ser string ou inteiro",
        ));
    };
    core_calculate_ibge_dv(&s).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Harmoniza um código municipal (de 6 ou 7 dígitos) para a representação canônica de 7 dígitos.
///
/// Aceita string ou inteiro (ex: `"355030"` ou `3550308`).
#[pyfunction]
pub fn harmonize_ibge_code(raw_code: &Bound<'_, PyAny>) -> PyResult<String> {
    let s = if let Ok(s) = raw_code.extract::<String>() {
        s
    } else if let Ok(n) = raw_code.extract::<i64>() {
        n.to_string()
    } else {
        return Err(PyValueError::new_err(
            "Código IBGE deve ser string ou inteiro",
        ));
    };
    core_harmonize_ibge_code(&s).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Valida se um código municipal do IBGE (de 6 ou 7 dígitos) é canônico e consistente com o algoritmo Módulo 10 (Luhn).
///
/// Aceita inteiros ou strings (ex: `3550308` ou `"3550308"`).
/// Retorna `True` se o código for válido e `False` caso contrário.
#[pyfunction]
pub fn validate_ibge_code(code: &Bound<'_, PyAny>) -> bool {
    if let Ok(s) = code.extract::<String>() {
        core_validate_ibge_code(&s)
    } else if let Ok(n) = code.extract::<i64>() {
        if n < 0 {
            false
        } else {
            core_validate_ibge_code(&n.to_string())
        }
    } else {
        false
    }
}

/// Reconcilia códigos municipais históricos com a malha canônica do IBGE de 2026.
///
/// Caso o código pertença a uma transição territorial histórica (ex: desmembramento do Tocantins
/// de Goiás em 1988 ou incorporação do Território Federal de Fernando de Noronha),
/// o código contemporâneo canônico é retornado.
///
/// Aceita string ou inteiro (ex: `"200001"`, `"520210"`, `200001`).
#[pyfunction]
#[pyo3(signature = (raw_code, reference_year=None))]
pub fn reconcile_historical_ibge_code(
    raw_code: &Bound<'_, PyAny>,
    reference_year: Option<u16>,
) -> PyResult<String> {
    let s = if let Ok(s) = raw_code.extract::<String>() {
        s
    } else if let Ok(n) = raw_code.extract::<i64>() {
        n.to_string()
    } else {
        return Err(PyValueError::new_err(
            "Código IBGE deve ser string ou inteiro",
        ));
    };
    core_reconcile_historical_ibge_code(&s, reference_year)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// Geoespacial Analítico (Uber H3 & Google S2)
// ---------------------------------------------------------------------------

/// Converte coordenadas de latitude e longitude em um índice hexagonal Uber H3.
#[pyfunction]
pub fn latlng_to_h3(lat: f64, lng: f64, resolution: u8) -> PyResult<u64> {
    coord_to_h3_index(lat, lng, resolution).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte um índice de célula H3 de 64 bits para o centróide em coordenadas (latitude, longitude).
#[pyfunction]
pub fn h3_to_latlng(h3_index: u64) -> PyResult<(f64, f64)> {
    core_h3_index_to_coord(h3_index).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte um índice de célula H3 de 64 bits para o centróide em coordenadas (latitude, longitude).
#[pyfunction]
pub fn h3_index_to_coord(h3_index: u64) -> PyResult<(f64, f64)> {
    h3_to_latlng(h3_index)
}

/// Retorna as células vizinhas em um disco espacial de raio k (anel/vizinhança de ordem k).
#[pyfunction]
pub fn h3_grid_disk(h3_index: u64, k: u32) -> PyResult<Vec<u64>> {
    core_h3_grid_disk(h3_index, k).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Calcula a distância de grade em número de células hexagonais entre duas posições H3 de mesma resolução.
#[pyfunction]
pub fn h3_grid_distance(origin: u64, destination: u64) -> PyResult<i32> {
    core_h3_grid_distance(origin, destination).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte coordenadas de latitude e longitude em um identificador S2 CellId de 64 bits.
#[pyfunction]
#[pyo3(signature = (lat, lon, level=None))]
pub fn latlng_to_s2(lat: f64, lon: f64, level: Option<u8>) -> PyResult<u64> {
    core_coord_to_s2_cell(lat, lon, level.unwrap_or(DEFAULT_S2_MUNICIPAL_LEVEL))
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Converte coordenadas de latitude e longitude em um identificador S2 CellId de 64 bits.
#[pyfunction]
#[pyo3(signature = (lat, lon, level=None))]
pub fn coord_to_s2_cell(lat: f64, lon: f64, level: Option<u8>) -> PyResult<u64> {
    latlng_to_s2(lat, lon, level)
}

/// Converte um identificador S2 CellId de volta para latitude e longitude centrais.
#[pyfunction]
pub fn s2_cell_to_coord(cell_id: u64) -> PyResult<(f64, f64)> {
    core_s2_cell_to_coord(cell_id).map_err(|e| PyValueError::new_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// Bioestatística e Análise de Mortalidade Prematura
// ---------------------------------------------------------------------------

/// Calcula Anos Potenciais de Vida Perdidos (APVP / YLL) para um conjunto de idades de óbito.
#[pyfunction]
#[pyo3(signature = (ages, cutoff_age=None))]
pub fn compute_apvp(ages: Vec<u16>, cutoff_age: Option<u16>) -> u64 {
    core_compute_apvp(&ages, cutoff_age.unwrap_or(70))
}

/// Calcula a taxa padronizada de APVP por 100.000 habitantes.
#[pyfunction]
pub fn compute_apvp_rate(total_apvp: u64, population: u64) -> PyResult<f64> {
    core_compute_apvp_rate(total_apvp, population).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Avalia vetorizadamente um RecordBatch Arrow e calcula métricas completas de APVP.
///
/// Libera o GIL durante o cálculo analítico intensivo sobre arrays Arrow.
#[pyfunction]
#[pyo3(signature = (wrapper, age_column, cutoff_age=None, reference_population=None))]
pub fn compute_batch_apvp<'py>(
    py: Python<'py>,
    wrapper: &RecordBatchWrapper,
    age_column: &str,
    cutoff_age: Option<u16>,
    reference_population: Option<u64>,
) -> PyResult<Bound<'py, PyDict>> {
    let batch = wrapper.batch.clone();
    let col = age_column.to_string();
    let cutoff = cutoff_age.unwrap_or(70);

    let metrics = py
        .allow_threads(move || core_compute_batch_apvp(&batch, &col, cutoff, reference_population))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let dict = PyDict::new_bound(py);
    dict.set_item("total_apvp", metrics.total_apvp)?;
    dict.set_item("premature_deaths", metrics.premature_deaths)?;
    dict.set_item(
        "mean_years_lost_per_death",
        metrics.mean_years_lost_per_death,
    )?;
    dict.set_item("cutoff_age", metrics.cutoff_age)?;
    dict.set_item("apvp_rate_per_100k", metrics.apvp_rate_per_100k)?;
    Ok(dict)
}

/// Calcula a Taxa Padronizada Direta de Mortalidade por 100.000 habitantes
/// utilizando a População Padrão Mundial da OMS (2000-2025).
///
/// Requer duas listas com exatamente 18 elementos para as faixas etárias quinquenais (0 a 85+ anos).
#[pyfunction]
pub fn compute_age_standardized_mortality_rate(
    observed_deaths: Vec<u64>,
    local_pop: Vec<u64>,
) -> PyResult<f64> {
    core_compute_age_standardized_mortality_rate(&observed_deaths, &local_pop)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// CSAP, Ontologias Médicas e Economia da Saúde
// ---------------------------------------------------------------------------

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

/// Retorna o título descritivo oficial de um grupo CSAP (1 a 19) segundo a Portaria MS/SAS nº 221/2008.
#[pyfunction]
pub fn csap_group_name(group_id: u8) -> PyResult<String> {
    let group = match group_id {
        1 => CsapGroup::Imunopreveniveis,
        2 => CsapGroup::Gastroenterites,
        3 => CsapGroup::Anemia,
        4 => CsapGroup::DeficienciasNutricionais,
        5 => CsapGroup::InfeccoesOuvidoNarizGarganta,
        6 => CsapGroup::PneumoniasBacterianas,
        7 => CsapGroup::Asma,
        8 => CsapGroup::DoencasPulmonares,
        9 => CsapGroup::Hipertensao,
        10 => CsapGroup::Angina,
        11 => CsapGroup::InsuficienciaCardiaca,
        12 => CsapGroup::DoencasCerebrovasculares,
        13 => CsapGroup::DiabetesMellitus,
        14 => CsapGroup::Epilepsias,
        15 => CsapGroup::InfeccaoTratoUrinario,
        16 => CsapGroup::InfeccoesPele,
        17 => CsapGroup::DoencaInflamatoriaPelvica,
        18 => CsapGroup::UlceraGastrointestinal,
        19 => CsapGroup::DoencasPreNatalParto,
        other => {
            return Err(PyValueError::new_err(format!(
                "Grupo CSAP inválido: {other}. Os grupos válidos são de 1 a 19."
            )))
        }
    };
    Ok(group.name().to_string())
}

/// Retorna os metadados do capítulo da CID-10 para o código informado (número, numeral romano e título em português).
#[pyfunction]
pub fn icd10_chapter<'py>(py: Python<'py>, code: &str) -> PyResult<Bound<'py, PyDict>> {
    let chapter = Icd10Chapter::from_code(code).ok_or_else(|| {
        PyValueError::new_err(format!(
            "Código CID-10 '{code}' inválido ou não reconhecido"
        ))
    })?;

    let dict = PyDict::new_bound(py);
    dict.set_item("number", chapter as u8)?;
    dict.set_item("roman", chapter.roman_numeral())?;
    dict.set_item("title_pt", chapter.title_pt())?;
    Ok(dict)
}

/// Valida a consistência biológica de um evento médico de acordo com sexo biológico e idade.
#[pyfunction]
pub fn validate_biological_consistency(icd10: &str, sex: &str, age_years: u16) -> PyResult<bool> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    let bio_sex = BiologicalSex::from_str_lenient(sex);
    match harmonizer.validate_biological_consistency(icd10, bio_sex, age_years) {
        Ok(()) => Ok(true),
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

/// Mapeia código histórico CID-9 para o equivalente canônico na CID-10.
#[pyfunction]
pub fn map_icd9_to_icd10(icd9: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd9_to_icd10(icd9)
}

/// Mapeia código canônico CID-10 para o equivalente histórico na CID-9.
#[pyfunction]
pub fn map_icd10_to_icd9(icd10: &str) -> Option<String> {
    let harmonizer = MedicalOntologyHarmonizer::new();
    harmonizer.map_icd10_to_icd9(icd10)
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

/// Verifica se um código do procedimento SIGTAP/SUS é uma amputação de membro.
#[pyfunction]
pub fn is_amputation_procedure(code: &str) -> bool {
    core_is_amputation_procedure(code)
}

/// Verifica se um código do procedimento SIGTAP/SUS é terapia renal substitutiva (diálise).
#[pyfunction]
pub fn is_dialysis_procedure(code: &str) -> bool {
    core_is_dialysis_procedure(code)
}

/// Faz parsing e validação estrutural de um código de procedimento SIGTAP de 10 dígitos.
#[pyfunction]
pub fn parse_sigtap_code(code: &str) -> PyResult<(u8, u8, u8, u16, u8)> {
    core_parse_sigtap_code(code)
        .map(|s| {
            (
                s.group,
                s.subgroup,
                s.form_of_organization,
                s.sequential,
                s.dv,
            )
        })
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Consulta a descrição e classe de um código ATC da OMS.
#[pyfunction]
pub fn lookup_atc(atc_code: &str) -> Option<String> {
    let harmonizer = PharmacyHarmonizer::new();
    harmonizer
        .lookup_atc(atc_code)
        .map(|d| d.active_ingredient.to_string())
}

/// Mapeia um código ATC da OMS para o conceito RxNorm internacional.
#[pyfunction]
pub fn map_atc_to_rxnorm(atc_code: &str) -> Option<String> {
    let harmonizer = PharmacyHarmonizer::new();
    harmonizer
        .map_atc_to_rxnorm(atc_code)
        .map(|s| s.to_string())
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

// ---------------------------------------------------------------------------
// Accessors Semânticos Especializados
// ---------------------------------------------------------------------------

/// Acessor semântico para Morbidade Hospitalar do SUS (SIH-SUS / RD3).
#[pyclass]
pub struct HospitalMorbidityAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl HospitalMorbidityAccessor {
    /// Ingestão e harmonização de AIH/SIH-SUS com suporte a anos e jurisdições múltiplas.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (jurisdiction=None, jurisdictions=None, year=None, years=None, month=None, harmonize_ibge=true, assign_h3=None, enrich_csap=true))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        jurisdiction: Option<String>,
        jurisdictions: Option<Vec<String>>,
        year: Option<u16>,
        years: Option<Vec<u16>>,
        month: Option<u8>,
        harmonize_ibge: bool,
        assign_h3: Option<u8>,
        enrich_csap: bool,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);

        let uf_list: Vec<Option<String>> = match (jurisdictions, jurisdiction) {
            (Some(ufs), _) => ufs.into_iter().map(Some).collect(),
            (None, Some(uf)) => vec![Some(uf)],
            (None, None) => vec![None],
        };

        let yr_list: Vec<u16> = match (years, year) {
            (Some(yrs), _) => yrs,
            (None, Some(yr)) => vec![yr],
            (None, None) => vec![2024],
        };

        if uf_list.len() == 1 && yr_list.len() == 1 {
            return engine_borrow.fetch(
                py,
                "datasus.sih",
                uf_list[0].clone(),
                yr_list[0],
                month,
                harmonize_ibge,
                assign_h3,
                enrich_csap,
                None,
            );
        }

        let mut all_batches = Vec::new();
        let mut last_manifest = None;

        for uf in &uf_list {
            for &yr in &yr_list {
                let res = engine_borrow.fetch(
                    py,
                    "datasus.sih",
                    uf.clone(),
                    yr,
                    month,
                    harmonize_ibge,
                    assign_h3,
                    enrich_csap,
                    None,
                )?;
                all_batches.push(res.batch);
                last_manifest = res.manifest;
            }
        }

        if all_batches.is_empty() {
            return engine_borrow.fetch(
                py,
                "datasus.sih",
                None,
                2024,
                month,
                harmonize_ibge,
                assign_h3,
                enrich_csap,
                None,
            );
        }

        let schema = all_batches[0].schema();
        let combined = py
            .allow_threads(|| concat_batches(&schema, &all_batches))
            .map_err(|e| PyValueError::new_err(format!("Erro ao concatenar lotes SIH: {e}")))?;

        Ok(RecordBatchWrapper::new(combined, last_manifest))
    }
}

/// Acessor semântico para Estatísticas Vitais (SIM e SINASC).
#[pyclass]
pub struct VitalStatisticsAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl VitalStatisticsAccessor {
    /// Ingestão e harmonização de SIM (Mortalidade) ou SINASC (Nascidos Vivos).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source="SIM", jurisdiction=None, year=2024, month=None, harmonize_ibge=true, assign_h3=None))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        source: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
        assign_h3: Option<u8>,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let src_id = match source.to_uppercase().as_str() {
            "SIM" => "datasus.sim",
            "SINASC" => "datasus.sinasc",
            other => {
                return Err(PyValueError::new_err(format!(
                    "Fonte vital desconhecida '{other}'. Use 'SIM' ou 'SINASC'"
                )))
            }
        };

        engine_borrow.fetch(
            py,
            src_id,
            jurisdiction,
            year,
            month,
            harmonize_ibge,
            assign_h3,
            false,
            None,
        )
    }
}

/// Acessor semântico para Notificações de Agravos (SINAN).
#[pyclass]
pub struct NotificationsAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl NotificationsAccessor {
    /// Ingestão e harmonização de notificações epidemiológicas SINAN (ex: dengue, tuberculose).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (disease="DENG", jurisdiction=None, year=2024, month=None, harmonize_ibge=true, assign_h3=None))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        disease: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
        assign_h3: Option<u8>,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let mut filters = HashMap::new();
        filters.insert("disease".into(), disease.to_uppercase());

        engine_borrow.fetch(
            py,
            "datasus.sinan",
            jurisdiction,
            year,
            month,
            harmonize_ibge,
            assign_h3,
            false,
            Some(filters),
        )
    }
}

/// Acessor semântico para Reanálise Climática e Atmosférica Global (Copernicus ERA5).
#[pyclass]
pub struct GlobalClimateAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl GlobalClimateAccessor {
    /// Ingestão de reanálise horária e diária ERA5 com suporte a malhas planetárias.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (jurisdiction=None, year=2024, month=None, grid_type=None))]
    pub fn fetch_reanalysis(
        &self,
        py: Python<'_>,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        grid_type: Option<String>,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let mut filters = HashMap::new();
        if let Some(gt) = grid_type {
            filters.insert("grid_type".into(), gt);
        }

        engine_borrow.fetch(
            py,
            "global.copernicus_era5",
            jurisdiction,
            year,
            month,
            false,
            None,
            false,
            Some(filters),
        )
    }
}

/// Acessor semântico para Dados Demográficos e Censitários do IBGE.
#[pyclass]
pub struct DemographicsAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl DemographicsAccessor {
    /// Ingestão de pesquisas do IBGE: censo, pnad, pof, pense ou munic.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source="censo", jurisdiction=None, year=2022, month=None, harmonize_ibge=true))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        source: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let src_id = match source.to_lowercase().as_str() {
            "censo" => "ibge.censo",
            "pnad" => "ibge.pnad",
            "pof" => "ibge.pof",
            "pense" => "ibge.pense",
            "munic" => "ibge.munic",
            other => {
                return Err(PyValueError::new_err(format!(
                    "Pesquisa IBGE desconhecida '{other}'. Opções: 'censo', 'pnad', 'pof', 'pense', 'munic'"
                )))
            }
        };

        engine_borrow.fetch(
            py,
            src_id,
            jurisdiction,
            year,
            month,
            harmonize_ibge,
            None,
            false,
            None,
        )
    }
}

/// Acessor semântico para Atenção Ambulatorial e Estabelecimentos (SIA-SUS e CNES).
#[pyclass]
pub struct AmbulatoryAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl AmbulatoryAccessor {
    /// Ingestão de SIA (Produção Ambulatorial) ou CNES (Cadastro de Estabelecimentos).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source="SIA", jurisdiction=None, year=2024, month=None, harmonize_ibge=true))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        source: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let src_id = match source.to_uppercase().as_str() {
            "SIA" | "SIASUS" => "datasus.sia",
            "CNES" => "datasus.cnes",
            other => {
                return Err(PyValueError::new_err(format!(
                    "Fonte ambulatorial desconhecida '{other}'. Use 'SIA' ou 'CNES'"
                )))
            }
        };

        engine_borrow.fetch(
            py,
            src_id,
            jurisdiction,
            year,
            month,
            harmonize_ibge,
            None,
            false,
            None,
        )
    }
}

/// Acessor semântico para Determinantes Ambientais e Climáticos Nacionais.
#[pyclass]
pub struct EnvironmentalAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl EnvironmentalAccessor {
    /// Ingestão de dados de ambiente: inmet, bdqueimadas, prodes ou sisagua.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source="inmet", jurisdiction=None, year=2024, month=None))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        source: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        let src_id = match source.to_lowercase().as_str() {
            "inmet" => "environmental.inmet",
            "bdqueimadas" | "queimadas" => "environmental.bdqueimadas",
            "prodes" | "desmatamento" => "environmental.prodes",
            "sisagua" | "agua" => "environmental.sisagua",
            other => {
                return Err(PyValueError::new_err(format!(
                    "Fonte ambiental desconhecida '{other}'. Opções: 'inmet', 'bdqueimadas', 'prodes', 'sisagua'"
                )))
            }
        };

        engine_borrow.fetch(
            py,
            src_id,
            jurisdiction,
            year,
            month,
            false,
            None,
            false,
            None,
        )
    }
}

/// Acessor semântico para Vulnerabilidade Social e CadÚnico (MDS).
#[pyclass]
pub struct SocialAccessor {
    engine: Py<Engine>,
}

#[pymethods]
impl SocialAccessor {
    /// Ingestão de microdados do Cadastro Único para Programas Sociais (CadÚnico).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (jurisdiction=None, year=2024, month=None, harmonize_ibge=true))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
    ) -> PyResult<RecordBatchWrapper> {
        let engine_borrow = self.engine.borrow(py);
        engine_borrow.fetch(
            py,
            "mds.cadunico",
            jurisdiction,
            year,
            month,
            harmonize_ibge,
            None,
            false,
            None,
        )
    }
}

// ---------------------------------------------------------------------------
// Gestão de Cache Colunar Hive-Parquet (CacheManager)
// ---------------------------------------------------------------------------

/// Gerenciador de cache local particionado Hive-Parquet do BRHealth.
#[pyclass]
pub struct CacheManager {
    app_service: Arc<BRHealthApplicationService>,
}

#[pymethods]
impl CacheManager {
    /// Limpa o cache Hive-Parquet.
    ///
    /// Se `source_id` for informado (ex: "datasus.sih"), limpa apenas a partição daquela fonte.
    /// Caso contrário, remove todo o diretório de cache local.
    /// Retorna a quantidade de diretórios/arquivos removidos.
    #[pyo3(signature = (source_id=None))]
    pub fn clear(&self, source_id: Option<&str>) -> PyResult<usize> {
        self.app_service
            .clear_cache(source_id)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Limpa dados de snapshots mais antigos que uma quantidade de dias ou data ISO.
    ///
    /// Retorna a contagem de snapshots removidos.
    #[pyo3(signature = (days=None, cutoff_date=None))]
    pub fn clear_older_than(
        &self,
        days: Option<i64>,
        cutoff_date: Option<&str>,
    ) -> PyResult<usize> {
        let cutoff = if let Some(iso_str) = cutoff_date {
            chrono::DateTime::parse_from_rfc3339(iso_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| PyValueError::new_err(format!("Data ISO 8601 inválida: {e}")))?
        } else if let Some(d) = days {
            chrono::Utc::now() - chrono::Duration::days(d)
        } else {
            return Err(PyValueError::new_err(
                "Informe 'days' (ex: 30) ou 'cutoff_date' em formato ISO (ex: '2025-01-01T00:00:00Z')",
            ));
        };

        self.app_service
            .clear_cache_older_than(cutoff)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Retorna um dicionário com estatísticas do cache (tamanho em bytes, contagem de snapshots, caminho).
    #[pyo3(signature = (source_id=None))]
    pub fn status<'py>(
        &self,
        py: Python<'py>,
        source_id: Option<&str>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let st = self
            .app_service
            .cache_status(source_id)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        dict.set_item("total_bytes", st.total_bytes)?;
        dict.set_item("snapshot_count", st.snapshot_count)?;
        dict.set_item("base_path", st.base_path)?;
        Ok(dict)
    }
}

// ---------------------------------------------------------------------------
// Motor Analítico BRHealth (Engine)
// ---------------------------------------------------------------------------

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

        let app_service = Arc::new(
            BRHealthApplicationService::standard_in_memory()
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        );

        Ok(Self {
            app_service,
            rt: Arc::new(rt),
        })
    }

    /// Acessor especializado para Morbidade Hospitalar do SUS (SIH-SUS).
    #[getter]
    pub fn hospital_morbidity(slf: PyRef<'_, Self>) -> HospitalMorbidityAccessor {
        HospitalMorbidityAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Estatísticas Vitais (SIM e SINASC).
    #[getter]
    pub fn vital_statistics(slf: PyRef<'_, Self>) -> VitalStatisticsAccessor {
        VitalStatisticsAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Notificações de Agravos Epidemiológicos (SINAN).
    #[getter]
    pub fn notifications(slf: PyRef<'_, Self>) -> NotificationsAccessor {
        NotificationsAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Reanálise Climática e Variáveis Ambientais (Copernicus ERA5).
    #[getter]
    pub fn global_climate(slf: PyRef<'_, Self>) -> GlobalClimateAccessor {
        GlobalClimateAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Demografia e Condições Censitárias (IBGE).
    #[getter]
    pub fn demographics(slf: PyRef<'_, Self>) -> DemographicsAccessor {
        DemographicsAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Assistência Ambulatorial e Estabelecimentos (SIA-SUS e CNES).
    #[getter]
    pub fn ambulatory(slf: PyRef<'_, Self>) -> AmbulatoryAccessor {
        AmbulatoryAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Clima e Determinantes Ambientais Nacionais (INMET, BDQueimadas, Prodes, Sisagua).
    #[getter]
    pub fn environmental(slf: PyRef<'_, Self>) -> EnvironmentalAccessor {
        EnvironmentalAccessor { engine: slf.into() }
    }

    /// Acessor especializado para Vulnerabilidade Social (CadÚnico / MDS).
    #[getter]
    pub fn social(slf: PyRef<'_, Self>) -> SocialAccessor {
        SocialAccessor { engine: slf.into() }
    }

    /// Sub-objeto de governança e gestão de cache Hive-Parquet.
    #[getter]
    pub fn cache(slf: PyRef<'_, Self>) -> CacheManager {
        CacheManager {
            app_service: slf.app_service.clone(),
        }
    }

    /// Retorna a lista de identificadores das fontes registradas.
    pub fn list_sources(&self) -> Vec<String> {
        self.app_service
            .registry()
            .list_all()
            .into_iter()
            .map(|meta| meta.id.to_string())
            .collect()
    }

    /// Retorna o número total de fontes registradas.
    pub fn source_count(&self) -> usize {
        self.app_service.registry().len()
    }

    /// Executa ingestão e harmonização de uma fonte de dados de saúde.
    ///
    /// Libera o GIL do interpretador Python durante o I/O assíncrono Tokio.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (source_id, jurisdiction=None, year=2024, month=None, harmonize_ibge=true, assign_h3=None, enrich_csap=false, extra_filters=None))]
    pub fn fetch(
        &self,
        py: Python<'_>,
        source_id: &str,
        jurisdiction: Option<String>,
        year: u16,
        month: Option<u8>,
        harmonize_ibge: bool,
        assign_h3: Option<u8>,
        enrich_csap: bool,
        extra_filters: Option<HashMap<String, String>>,
    ) -> PyResult<RecordBatchWrapper> {
        let params = DataQueryParams {
            scope: GeographicScope::National {
                iso_3166_alpha3: "BRA".into(),
            },
            jurisdiction_code: jurisdiction,
            year,
            month,
            extra_filters: extra_filters.unwrap_or_default(),
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
            custom_steps: Vec::new(),
        };

        let app = self.app_service.clone();
        let rt = self.rt.clone();
        let source_id_str = source_id.to_string();

        let result = py
            .allow_threads(move || {
                rt.block_on(async move {
                    app.execute_full_pipeline(&source_id_str, &params, &options)
                        .await
                })
            })
            .map_err(|err| match err {
                brhealth_core::domain::ports::outbound::PortError::ResourceNotFound(msg) => {
                    SourceNotFoundError::new_err(msg)
                }
                brhealth_core::domain::ports::outbound::PortError::TransportError(msg) => {
                    TransportError::new_err(msg)
                }
                brhealth_core::domain::ports::outbound::PortError::ValidationError(msg)
                | brhealth_core::domain::ports::outbound::PortError::SchemaMismatch(msg) => {
                    ValidationError::new_err(msg)
                }
                other => BRHealthError::new_err(other.to_string()),
            })?;

        let combined_batch = if result.batches.is_empty() {
            let src = self
                .app_service
                .registry()
                .get(source_id)
                .map_err(|e| SourceNotFoundError::new_err(e.to_string()))?;
            RecordBatch::new_empty(src.target_schema())
        } else {
            result.batches[0].clone()
        };

        Ok(RecordBatchWrapper::new(
            combined_batch,
            Some(result.manifest),
        ))
    }

    /// Avalia um RecordBatch de internações hospitalares e calcula métricas de CSAP.
    ///
    /// Libera o GIL durante os cálculos de bioestatística e saúde coletiva.
    #[pyo3(signature = (wrapper, reference_population=None))]
    pub fn evaluate_csap<'py>(
        &self,
        py: Python<'py>,
        wrapper: &RecordBatchWrapper,
        reference_population: Option<u64>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let batch = wrapper.batch.clone();
        let metrics = py
            .allow_threads(move || compute_csap_metrics(&batch, reference_population))
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
        let bio_sex = BiologicalSex::from_str_lenient(sex);
        match harmonizer.validate_biological_consistency(icd10, bio_sex, age_years) {
            Ok(()) => Ok(true),
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    }

    /// Valida se um código municipal do IBGE é canônico e consistente com o algoritmo Módulo 10 (Luhn).
    pub fn validate_ibge_code(&self, code: &Bound<'_, PyAny>) -> bool {
        validate_ibge_code(code)
    }

    /// Retorna a versão do motor analítico.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// ---------------------------------------------------------------------------
// Singleton Global e Função Top-Level fetch()
// ---------------------------------------------------------------------------

static GLOBAL_ENGINE: OnceLock<Engine> = OnceLock::new();

fn get_global_engine() -> PyResult<&'static Engine> {
    if let Some(engine) = GLOBAL_ENGINE.get() {
        return Ok(engine);
    }
    let engine = Engine::new()?;
    let _ = GLOBAL_ENGINE.set(engine);
    GLOBAL_ENGINE
        .get()
        .ok_or_else(|| PyValueError::new_err("Falha ao inicializar o motor analítico global"))
}

/// Executa ingestão e harmonização de qualquer fonte registrada usando o motor global do BRHealth.
///
/// Libera o GIL do interpretador durante todo o pipeline.
#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (source_id, jurisdiction=None, year=2024, month=None, harmonize_ibge=true, assign_h3=None, enrich_csap=false, extra_filters=None))]
pub fn fetch(
    py: Python<'_>,
    source_id: &str,
    jurisdiction: Option<String>,
    year: u16,
    month: Option<u8>,
    harmonize_ibge: bool,
    assign_h3: Option<u8>,
    enrich_csap: bool,
    extra_filters: Option<HashMap<String, String>>,
) -> PyResult<RecordBatchWrapper> {
    let engine = get_global_engine()?;
    engine.fetch(
        py,
        source_id,
        jurisdiction,
        year,
        month,
        harmonize_ibge,
        assign_h3,
        enrich_csap,
        extra_filters,
    )
}

/// Diagnóstico do ambiente Python: verifica a disponibilidade das bibliotecas 'pyarrow', 'polars', 'pandas' e 'torch'.
#[pyfunction]
pub fn check_environment<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new_bound(py);
    dict.set_item("pyarrow", py.import_bound("pyarrow").is_ok())?;
    dict.set_item("polars", py.import_bound("polars").is_ok())?;
    dict.set_item("pandas", py.import_bound("pandas").is_ok())?;
    dict.set_item("torch", py.import_bound("torch").is_ok())?;
    Ok(dict)
}

// ---------------------------------------------------------------------------
// Registro de Módulo PyO3
// ---------------------------------------------------------------------------

/// Módulo Python do BRHealth.
#[pymodule]
fn brhealth(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Exceções Tipadas
    m.add("BRHealthError", m.py().get_type_bound::<BRHealthError>())?;
    m.add(
        "SourceNotFoundError",
        m.py().get_type_bound::<SourceNotFoundError>(),
    )?;
    m.add("TransportError", m.py().get_type_bound::<TransportError>())?;
    m.add(
        "ValidationError",
        m.py().get_type_bound::<ValidationError>(),
    )?;

    // Diagnóstico
    m.add_function(wrap_pyfunction!(check_environment, m)?)?;

    // Decodificadores
    m.add_function(wrap_pyfunction!(read_dbc, m)?)?;
    m.add_function(wrap_pyfunction!(read_dbf, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_dbc, m)?)?;

    // Harmonização Territorial IBGE
    m.add_function(wrap_pyfunction!(calculate_ibge_dv, m)?)?;
    m.add_function(wrap_pyfunction!(harmonize_ibge_code, m)?)?;
    m.add_function(wrap_pyfunction!(validate_ibge_code, m)?)?;
    m.add_function(wrap_pyfunction!(reconcile_historical_ibge_code, m)?)?;

    // Geoespacial
    m.add_function(wrap_pyfunction!(latlng_to_h3, m)?)?;
    m.add_function(wrap_pyfunction!(h3_to_latlng, m)?)?;
    m.add_function(wrap_pyfunction!(h3_index_to_coord, m)?)?;
    m.add_function(wrap_pyfunction!(h3_grid_disk, m)?)?;
    m.add_function(wrap_pyfunction!(h3_grid_distance, m)?)?;
    m.add_function(wrap_pyfunction!(latlng_to_s2, m)?)?;
    m.add_function(wrap_pyfunction!(coord_to_s2_cell, m)?)?;
    m.add_function(wrap_pyfunction!(s2_cell_to_coord, m)?)?;

    // Bioestatística & Mortalidade
    m.add_function(wrap_pyfunction!(compute_apvp, m)?)?;
    m.add_function(wrap_pyfunction!(compute_apvp_rate, m)?)?;
    m.add_function(wrap_pyfunction!(compute_batch_apvp, m)?)?;
    m.add_function(wrap_pyfunction!(
        compute_age_standardized_mortality_rate,
        m
    )?)?;

    // CSAP, Ontologias & Farmácia
    m.add_function(wrap_pyfunction!(classify_cid10, m)?)?;
    m.add_function(wrap_pyfunction!(is_csap, m)?)?;
    m.add_function(wrap_pyfunction!(csap_group_name, m)?)?;
    m.add_function(wrap_pyfunction!(icd10_chapter, m)?)?;
    m.add_function(wrap_pyfunction!(validate_biological_consistency, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd9_to_icd10, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd10_to_icd9, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd10_to_icd11, m)?)?;
    m.add_function(wrap_pyfunction!(map_icd10_to_snomed, m)?)?;
    m.add_function(wrap_pyfunction!(is_amputation_procedure, m)?)?;
    m.add_function(wrap_pyfunction!(is_dialysis_procedure, m)?)?;
    m.add_function(wrap_pyfunction!(parse_sigtap_code, m)?)?;
    m.add_function(wrap_pyfunction!(lookup_atc, m)?)?;
    m.add_function(wrap_pyfunction!(map_atc_to_rxnorm, m)?)?;
    m.add_function(wrap_pyfunction!(compute_roi, m)?)?;

    // Ingestão Top-Level
    m.add_function(wrap_pyfunction!(fetch, m)?)?;

    // Classes & Accessors
    m.add_class::<Engine>()?;
    m.add_class::<CacheManager>()?;
    m.add_class::<RecordBatchWrapper>()?;
    m.add_class::<HospitalMorbidityAccessor>()?;
    m.add_class::<VitalStatisticsAccessor>()?;
    m.add_class::<NotificationsAccessor>()?;
    m.add_class::<GlobalClimateAccessor>()?;
    m.add_class::<DemographicsAccessor>()?;
    m.add_class::<AmbulatoryAccessor>()?;
    m.add_class::<EnvironmentalAccessor>()?;
    m.add_class::<SocialAccessor>()?;

    Ok(())
}
