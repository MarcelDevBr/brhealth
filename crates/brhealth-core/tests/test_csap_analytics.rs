// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Testes de integração para analítica de CSAP (Portaria 221/2008) e Economia da Saúde.

use std::sync::Arc;

use arrow::array::{BooleanArray, Float64Array, StringArray, UInt8Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use brhealth_core::domain::analytics::csap::{
    compute_csap_metrics, compute_primary_care_roi, enrich_sih_batch_with_csap, is_csap,
};

#[test]
fn test_csap_end_to_end_pipeline() {
    let diagnoses = vec![
        Some("J450"), // Asma (CSAP Grupo 7)
        Some("I10"),  // Hipertensão (CSAP Grupo 9)
        Some("E101"), // Diabetes (CSAP Grupo 13)
        Some("A09"),  // Gastroenterite (CSAP Grupo 2)
        Some("S020"), // Fratura do crânio (Não CSAP)
        Some("C349"), // Neoplasia de pulmão (Não CSAP)
        Some("K250"), // Úlcera (CSAP Grupo 18)
    ];

    let costs = vec![
        Some(450.0),
        Some(320.0),
        Some(800.0),
        Some(250.0),
        Some(5000.0),
        Some(12000.0),
        Some(900.0),
    ];

    let lengths_of_stay = vec![
        Some(2),
        Some(1),
        Some(4),
        Some(2),
        Some(15),
        Some(20),
        Some(5),
    ];

    let schema = Arc::new(Schema::new(vec![
        Field::new("primary_diagnosis", DataType::Utf8, true),
        Field::new("total_cost", DataType::Float64, true),
        Field::new("length_of_stay", DataType::UInt8, true),
    ]));

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(diagnoses)),
            Arc::new(Float64Array::from(costs)),
            Arc::new(UInt8Array::from(lengths_of_stay)),
        ],
    )
    .unwrap();

    // 1. Enriquecimento de batch
    let enriched = enrich_sih_batch_with_csap(&batch).unwrap();
    assert_eq!(enriched.num_rows(), 7);
    assert_eq!(enriched.num_columns(), 6);

    let is_csap_col = enriched
        .column_by_name("is_csap")
        .unwrap()
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(is_csap_col.value(0)); // J450
    assert!(is_csap_col.value(1)); // I10
    assert!(is_csap_col.value(2)); // E101
    assert!(is_csap_col.value(3)); // A09
    assert!(!is_csap_col.value(4)); // S020
    assert!(!is_csap_col.value(5)); // C349
    assert!(is_csap_col.value(6)); // K250

    // 2. Cálculo de Métricas Bioestatísticas
    // População de 50.000 hab.
    let metrics = compute_csap_metrics(&batch, Some(50_000)).unwrap();
    assert_eq!(metrics.total_admissions, 7);
    assert_eq!(metrics.csap_admissions, 5);

    // 5 internações em 50.000 hab = 1.0 por 10.000 hab
    let rate = metrics.csap_rate_per_10k.unwrap();
    assert!((rate - 1.0).abs() < 1e-6);

    // Custo evitável: 450 + 320 + 800 + 250 + 900 = 2720.0
    assert!((metrics.avoidable_cost - 2720.0).abs() < 1e-6);

    // Dias evitáveis: 2 + 1 + 4 + 2 + 5 = 14
    assert_eq!(metrics.avoidable_days, 14);

    // 3. Avaliação de ROI da Atenção Primária
    // Com fração de impacto de 40% sobre R$ 2.720 = R$ 1.088
    // Investimento na UBS: R$ 500
    // ROI = (1088 - 500) / 500 = 588 / 500 = 1.176 (117,6% de retorno)
    let roi = compute_primary_care_roi(metrics.avoidable_cost, 500.0, 0.40).unwrap();
    assert!((roi - 1.176).abs() < 1e-6);
}

#[test]
fn test_is_csap_quick_lookup() {
    assert!(is_csap("J45"));
    assert!(is_csap("I10"));
    assert!(is_csap("A04"));
    assert!(!is_csap("K35")); // Apendicite aguda (cirúrgica, não sensível à atenção primária ambulatorial)
}
