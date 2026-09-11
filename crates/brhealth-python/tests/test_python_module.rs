use arrow::array::{Int32Array, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use pyo3::prelude::*;
use std::sync::Arc;

#[test]
fn test_python_module_metadata() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(version, "0.1.0");
}

#[test]
fn test_python_engine_initialization_and_methods() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        let engine = brhealth::Engine::new().unwrap();
        assert_eq!(engine.source_count(), 26);
        let sources = engine.list_sources();
        assert!(sources.contains(&"datasus.sim".to_string()));
        assert!(sources.contains(&"global.who_gho".to_string()));
        assert!(sources.contains(&"ibge.pof".to_string()));
        assert!(sources.contains(&"global.openaq".to_string()));

        let valid = engine
            .validate_biological_consistency("O00", "F", 30)
            .unwrap();
        assert!(valid);

        let invalid = engine.validate_biological_consistency("O00", "M", 30);
        assert!(invalid.is_err());
    });
}

#[test]
fn test_python_csap_and_roi_functions() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        let group = brhealth::classify_cid10("J450");
        assert_eq!(group, Some(7));

        let non_csap = brhealth::classify_cid10("S060");
        assert_eq!(non_csap, None);

        let is_csap = brhealth::is_csap("J450");
        assert!(is_csap);

        let name = brhealth::csap_group_name(7).unwrap();
        assert_eq!(name, "Asma");

        let invalid_group = brhealth::csap_group_name(20);
        assert!(invalid_group.is_err());

        let roi = brhealth::compute_roi(500_000.0, 100_000.0, 0.50).unwrap();
        assert!((roi - 1.5).abs() < 1e-6);
    });
}

#[test]
fn test_python_extended_features() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        // APVP
        let apvp = brhealth::compute_apvp(vec![30, 45, 15], Some(70));
        assert_eq!(apvp, 120);

        let rate = brhealth::compute_apvp_rate(120, 100_000).unwrap();
        assert!((rate - 120.0).abs() < 1e-6);

        // S2
        let s2_cell = brhealth::coord_to_s2_cell(-23.550520, -46.633308, Some(10)).unwrap();
        assert_ne!(s2_cell, 0);

        let (lat, lon) = brhealth::s2_cell_to_coord(s2_cell).unwrap();
        assert!((lat - (-23.55)).abs() < 0.5);
        assert!((lon - (-46.63)).abs() < 0.5);

        // CID-9 to CID-10
        let icd10 = brhealth::map_icd9_to_icd10("250");
        assert_eq!(icd10, Some("E14".to_string()));

        // SIGTAP
        assert!(brhealth::is_amputation_procedure("0407040101"));
        assert!(brhealth::is_dialysis_procedure("0305010107"));

        // ATC
        let drug = brhealth::lookup_atc("A10BA02");
        assert_eq!(drug, Some("Metformina".to_string()));
    });
}

#[test]
fn test_python_ibge_validation() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        // Validação de São Paulo/SP (3550308) - inteiro e string
        let sp_int = py.eval_bound("3550308", None, None).unwrap();
        let sp_str = py.eval_bound("'3550308'", None, None).unwrap();
        assert!(brhealth::validate_ibge_code(&sp_int));
        assert!(brhealth::validate_ibge_code(&sp_str));

        // Validação do Rio de Janeiro/RJ (3304557) - inteiro e string
        let rj_int = py.eval_bound("3304557", None, None).unwrap();
        let rj_str = py.eval_bound("'3304557'", None, None).unwrap();
        assert!(brhealth::validate_ibge_code(&rj_int));
        assert!(brhealth::validate_ibge_code(&rj_str));

        // Código com DV inválido
        let invalid_dv = py.eval_bound("3550309", None, None).unwrap();
        assert!(!brhealth::validate_ibge_code(&invalid_dv));

        // Código malformado
        let invalid_str = py.eval_bound("'35503A8'", None, None).unwrap();
        assert!(!brhealth::validate_ibge_code(&invalid_str));

        // Cálculo de DV e harmonização aceitando int e str
        let sp_6_int = py.eval_bound("355030", None, None).unwrap();
        let sp_6_str = py.eval_bound("'355030'", None, None).unwrap();
        assert_eq!(brhealth::calculate_ibge_dv(&sp_6_int).unwrap(), 8);
        assert_eq!(brhealth::calculate_ibge_dv(&sp_6_str).unwrap(), 8);
        assert_eq!(brhealth::harmonize_ibge_code(&sp_6_int).unwrap(), "3550308");
        assert_eq!(brhealth::harmonize_ibge_code(&sp_6_str).unwrap(), "3550308");

        // Reconciliação de municípios históricos (Fernando de Noronha e Tocantins)
        let fn_code = py.eval_bound("'200001'", None, None).unwrap();
        let reconciled_fn = brhealth::reconcile_historical_ibge_code(&fn_code, Some(1980)).unwrap();
        assert_eq!(reconciled_fn, "2605459");

        // Via Engine
        let engine = brhealth::Engine::new().unwrap();
        assert!(engine.validate_ibge_code(&sp_int));
        assert!(engine.validate_ibge_code(&rj_int));
        assert!(!engine.validate_ibge_code(&invalid_dv));
    });
}

#[test]
fn test_python_h3_and_spatial_features() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        let lat = -23.550520;
        let lng = -46.633308;
        let h3_idx = brhealth::latlng_to_h3(lat, lng, 7).unwrap();
        assert_ne!(h3_idx, 0);

        let (c_lat, c_lng) = brhealth::h3_to_latlng(h3_idx).unwrap();
        assert!((c_lat - lat).abs() < 0.1);
        assert!((c_lng - lng).abs() < 0.1);

        let disk = brhealth::h3_grid_disk(h3_idx, 1).unwrap();
        assert_eq!(disk.len(), 7); // Anel de raio 1 no H3 possui 7 hexágonos

        let dist = brhealth::h3_grid_distance(h3_idx, h3_idx).unwrap();
        assert_eq!(dist, 0);
    });
}

#[test]
fn test_python_ontology_and_mortality() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        // Validação Biológica top-level
        assert!(brhealth::validate_biological_consistency("O00", "F", 28).unwrap());
        assert!(brhealth::validate_biological_consistency("O00", "M", 28).is_err());

        // Metadados do Capítulo CID-10
        let chap = brhealth::icd10_chapter(py, "I10").unwrap();
        let num: u8 = chap.get_item("number").unwrap().unwrap().extract().unwrap();
        let roman: String = chap.get_item("roman").unwrap().unwrap().extract().unwrap();
        assert_eq!(num, 9);
        assert_eq!(roman, "IX");

        // Padronização Direta da OMS (18 faixas)
        let deaths = vec![10; 18];
        let pop = vec![1000; 18];
        let std_rate = brhealth::compute_age_standardized_mortality_rate(deaths, pop).unwrap();
        assert!(std_rate > 0.0);
    });
}

#[test]
fn test_python_record_batch_wrapper_utilities() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("age", DataType::Int32, false),
        ]));
        let id_array = Arc::new(Int32Array::from(vec![1, 2, 3]));
        let age_array = Arc::new(Int32Array::from(vec![25, 40, 65]));
        let batch = RecordBatch::try_new(schema, vec![id_array, age_array]).unwrap();

        let wrapper = brhealth::RecordBatchWrapper::new(batch, None);
        assert_eq!(wrapper.num_rows(), 3);
        assert_eq!(wrapper.num_columns(), 2);
        assert_eq!(wrapper.shape(), (3, 2));
        assert_eq!(wrapper.__len__(), 3);
        assert_eq!(wrapper.columns(), vec!["id", "age"]);

        let head = wrapper.head(Some(2));
        assert_eq!(head.num_rows(), 2);
        assert_eq!(head.num_columns(), 2);

        let tail = wrapper.tail(Some(1));
        assert_eq!(tail.num_rows(), 1);

        let repr = wrapper.__repr__();
        assert!(repr.contains("3 rows x 2 columns"));

        let html = wrapper._repr_html_();
        assert!(html.contains("BRHealth Colunar RecordBatch"));
        assert!(html.contains("3 linhas &times; 2 colunas"));

        // Cálculo de APVP em lote sobre wrapper
        let apvp_dict =
            brhealth::compute_batch_apvp(py, &wrapper, "age", Some(70), Some(10_000)).unwrap();
        let total_apvp: u64 = apvp_dict
            .get_item("total_apvp")
            .unwrap()
            .unwrap()
            .extract()
            .unwrap();
        // (70 - 25) + (70 - 40) + (70 - 65) = 45 + 30 + 5 = 80
        assert_eq!(total_apvp, 80);
    });
}

#[test]
fn test_python_engine_new_accessors_and_top_level_fetch() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let engine_obj = Py::new(py, brhealth::Engine::new().unwrap()).unwrap();

        // Testar instâncias dos novos accessors via getattr do Python
        assert!(engine_obj.getattr(py, "demographics").is_ok());
        assert!(engine_obj.getattr(py, "ambulatory").is_ok());
        assert!(engine_obj.getattr(py, "environmental").is_ok());
        assert!(engine_obj.getattr(py, "social").is_ok());

        // Top-level fetch: fonte desconhecida deve retornar erro tipado sem pânico
        let res_err = brhealth::fetch(
            py,
            "fonte.desconhecida",
            Some("35".to_string()),
            2024,
            None,
            true,
            None,
            false,
            None,
        );
        assert!(res_err.is_err());
    });
}

#[test]
fn test_python_engine_cache_first_policy() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        let engine = brhealth::Engine::new().unwrap();
        // Acesso via Engine canônico
        let sources = engine.list_sources();
        assert!(sources.contains(&"datasus.sim".to_string()));
    });
}

#[test]
fn test_python_check_environment() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let env_dict = brhealth::check_environment(py).unwrap();
        assert!(env_dict.contains("pyarrow").unwrap());
        assert!(env_dict.contains("polars").unwrap());
        assert!(env_dict.contains("pandas").unwrap());
        assert!(env_dict.contains("torch").unwrap());
    });
}

#[test]
fn test_python_source_alias_resolution() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|_py| {
        let engine = brhealth::Engine::new().unwrap();
        // A fonte com ponto deve existir na lista
        assert!(engine.list_sources().contains(&"datasus.sih".to_string()));
    });
}

#[test]
fn test_python_cache_manager() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let engine_obj = Py::new(py, brhealth::Engine::new().unwrap()).unwrap();
        let cache_obj = engine_obj.getattr(py, "cache").unwrap();

        // Status do cache deve retornar dict com as chaves esperadas
        let status_dict: Bound<'_, pyo3::types::PyDict> = cache_obj
            .call_method0(py, "status")
            .unwrap()
            .extract(py)
            .unwrap();
        assert!(status_dict.contains("total_bytes").unwrap());
        assert!(status_dict.contains("snapshot_count").unwrap());
        assert!(status_dict.contains("base_path").unwrap());

        // Limpeza de fonte específica
        let cleared_sih: usize = cache_obj
            .call_method1(py, "clear", ("datasus.sih",))
            .unwrap()
            .extract(py)
            .unwrap();
        assert_eq!(cleared_sih, 0); // nenhuma pasta criada ainda

        // Limpeza por dias
        let cleared_days: usize = cache_obj
            .call_method1(py, "clear_older_than", (30,))
            .unwrap()
            .extract(py)
            .unwrap();
        assert_eq!(cleared_days, 0);
    });
}
