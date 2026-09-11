// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Suíte de Micro-benchmarks de Alto Desempenho com Criterion.
//!
//! Avalia a performance dos módulos críticos de CPU do motor BRHealth:
//! - Cálculo de Dígito Verificador IBGE (Luhn Módulo 10).
//! - Indexação Espacial Hexagonal Uber H3.
//! - Classificação e Enriquecimento Analítico de CSAP (Portaria 221/2008).
//! - Descompressão nativa DATASUS Blast PKWARE DCL.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use brhealth_core::decoders::blast::BlastDecompressor;
use brhealth_core::domain::analytics::csap::classify_cid10;
use brhealth_core::domain::spatial::coord_to_h3_index;
use brhealth_core::domain::transforms::ibge::calculate_ibge_dv;

fn bench_ibge_dv(c: &mut Criterion) {
    let mut group = c.benchmark_group("ibge_transforms");
    group.bench_function("calculate_ibge_dv_sao_paulo", |b| {
        b.iter(|| calculate_ibge_dv(black_box("355030")))
    });
    group.finish();
}

fn bench_h3_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_h3");
    group.bench_function("coord_to_h3_res8", |b| {
        b.iter(|| coord_to_h3_index(black_box(-23.55052), black_box(-46.633308), black_box(8)))
    });
    group.finish();
}

fn bench_csap_classification(c: &mut Criterion) {
    let mut group = c.benchmark_group("csap_analytics");
    let cids = ["J450", "I10", "E119", "A09", "S060", "N390", "K250", "C500"];
    group.bench_function("classify_cid10_batch", |b| {
        b.iter(|| {
            for cid in &cids {
                let _ = classify_cid10(black_box(cid));
            }
        })
    });
    group.finish();
}

fn bench_blast_noop(c: &mut Criterion) {
    let decompressor = BlastDecompressor::default();
    let mut group = c.benchmark_group("blast_decoder");
    // Header PKWARE DCL válido com stream vazio
    let empty_stream = [0u8, 4u8, 0u8];
    group.bench_function("blast_empty_stream", |b| {
        b.iter(|| {
            let _ = decompressor.decompress(black_box(&empty_stream));
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_ibge_dv,
    bench_h3_indexing,
    bench_csap_classification,
    bench_blast_noop
);
criterion_main!(benches);
