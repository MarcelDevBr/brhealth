// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Exemplo de Avaliação Econômica da Saúde e Vigilância de CSAP:
//! 1. Geração de coorte sintética de morbidade hospitalar (SIH/AIH).
//! 2. Classificação colunar conforme a Portaria MS/SAS nº 221/2008 (19 Grupos).
//! 3. Apuração de Taxa de Internação por 10.000 habitantes.
//! 4. Quantificação de Custos Hospitalares Evitáveis e Diárias Evitáveis.
//! 5. Modelagem de Retorno sobre o Investimento (ROI) com fortalecimento da ESF.

use std::sync::Arc;

use arrow::array::{Float64Array, StringArray, UInt8Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use brhealth_core::domain::analytics::csap::{
    compute_csap_metrics, compute_primary_care_roi, enrich_sih_batch_with_csap,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BRHealth: Avaliação Econômica e CSAP (Portaria 221/2008) ===");

    // 1. Simular coorte hospitalar com diagnósticos principais (CID-10), custos (VAL_TOT) e permanência
    let cids = Arc::new(StringArray::from(vec![
        Some("J450"), // CSAP Grupo 7: Asma
        Some("I10"),  // CSAP Grupo 9: Hipertensão
        Some("E119"), // CSAP Grupo 13: Diabetes Mellitus
        Some("A09"),  // CSAP Grupo 2: Gastroenterites
        Some("J14"),  // CSAP Grupo 6: Pneumonias bacterianas
        Some("S060"), // Não-CSAP: Traumatismo intracraniano
        Some("C509"), // Não-CSAP: Neoplasia de mama
    ]));

    let costs = Arc::new(Float64Array::from(vec![
        Some(480.0),
        Some(350.0),
        Some(920.0),
        Some(260.0),
        Some(1150.0),
        Some(8500.0),
        Some(14200.0),
    ]));

    let days = Arc::new(UInt8Array::from(vec![
        Some(2),
        Some(1),
        Some(4),
        Some(2),
        Some(5),
        Some(12),
        Some(18),
    ]));

    let schema = Arc::new(Schema::new(vec![
        Field::new("primary_diagnosis", DataType::Utf8, true),
        Field::new("total_cost", DataType::Float64, true),
        Field::new("length_of_stay", DataType::UInt8, true),
    ]));

    let batch = RecordBatch::try_new(schema, vec![cids, costs, days])?;

    // 2. Enriquecimento colunar com grupos canônicos
    let enriched = enrich_sih_batch_with_csap(&batch)?;
    let col_names: Vec<String> = enriched
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();
    println!("-> Lote enriquecido com colunas: {:?}", col_names);

    // 3. Métricas Bioestatísticas (considerando município de 40.000 habitantes)
    let population = 40_000u64;
    let metrics = compute_csap_metrics(&batch, Some(population))?;

    println!("\n--- Indicadores Epidemiológicos ---");
    println!("Total de Internações: {}", metrics.total_admissions);
    println!(
        "Internações por CSAP: {} ({:.1}%)",
        metrics.csap_admissions,
        metrics.csap_proportion * 100.0
    );
    if let Some(rate) = metrics.csap_rate_per_10k {
        println!("Taxa de CSAP por 10.000 hab: {:.2}", rate);
    }
    println!("Custo Hospitalar Total: R$ {:.2}", metrics.total_cost);
    println!(
        "Custo Hospitalar Evitável: R$ {:.2}",
        metrics.avoidable_cost
    );
    println!(
        "Diárias Hospitalares Evitáveis: {} dias",
        metrics.avoidable_days
    );

    // 4. Modelagem Econométrica de ROI na Atenção Básica
    // Investimento de R$ 1.000,00 na UBS para acompanhamento contínuo
    // Fração atribuível evitável de 45% (alpha = 0.45)
    let investment = 1000.0;
    let alpha = 0.45;
    let roi = compute_primary_care_roi(metrics.avoidable_cost, investment, alpha)?;

    let expected_savings = alpha * metrics.avoidable_cost;
    let net_return = expected_savings - investment;

    println!("\n--- Avaliação Econômica e ROI da Atenção Primária ---");
    println!(
        "Economia Hospitalar Projetada (alpha={:.0}%): R$ {:.2}",
        alpha * 100.0,
        expected_savings
    );
    println!(
        "Investimento Incremental na Atenção Básica: R$ {:.2}",
        investment
    );
    println!("Retorno Líquido para o SUS: R$ {:.2}", net_return);
    println!("ROI da Atenção Primária: {:.1}%", roi * 100.0);

    println!("\n=== Análise concluída com conformidade bioestatística estrita! ===");
    Ok(())
}
