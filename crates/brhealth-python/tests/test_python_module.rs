use pyo3::prelude::*;

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

        // Via Engine
        let engine = brhealth::Engine::new().unwrap();
        assert!(engine.validate_ibge_code(&sp_int));
        assert!(engine.validate_ibge_code(&rj_int));
        assert!(!engine.validate_ibge_code(&invalid_dv));
    });
}
