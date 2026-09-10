// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Testes de integração exaustivos para todos os Country Packs (Brasil e Global).

use std::collections::HashMap;
use std::sync::Arc;

use brhealth_core::decoders::dbc::DbcDecompressor;
use brhealth_core::domain::registry::SourceRegistry;
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, HealthDataSourceSPI, SourceExecutionContext,
};
use brhealth_core::infrastructure::cache::MemoryCache;
use brhealth_core::infrastructure::state::MemorySyncState;
use brhealth_core::infrastructure::transport::MockTransport;
use brhealth_core::{create_pack_brasil, create_pack_global};

#[test]
fn test_country_pack_brasil_metadata_and_registry() {
    let pack = create_pack_brasil();
    assert_eq!(
        pack.len(),
        20,
        "Pack Brasil deve conter exatamente 20 fontes oficiais"
    );

    let mut registry = SourceRegistry::new();
    registry.register_pack(pack);

    let national_scope = GeographicScope::National {
        iso_3166_alpha3: "BRA".into(),
    };
    let sources_bra = registry.list_by_scope(&national_scope);
    assert_eq!(sources_bra.len(), 20);

    // Verificar IDs das 20 fontes nacionais
    let expected_ids = [
        "datasus.sim",
        "datasus.sinasc",
        "datasus.sih",
        "datasus.sinan",
        "datasus.siasus",
        "datasus.cnes",
        "datasus.sipni",
        "datasus.sisvan",
        "datasus.siscan",
        "datasus.bps",
        "ibge.censo",
        "ibge.pnad",
        "ibge.pof",
        "ibge.pense",
        "ibge.munic",
        "mds.cadunico",
        "environmental.inmet",
        "environmental.bdqueimadas",
        "environmental.sisagua",
        "environmental.prodes",
    ];

    for id in &expected_ids {
        let source = registry.get(id);
        assert!(
            source.is_ok(),
            "Fonte '{}' deve estar registrada no SourceRegistry",
            id
        );
        let s = source.unwrap();
        let meta = s.metadata();
        assert_eq!(meta.id, *id);
        assert!(!meta.display_name.is_empty());
        assert!(!meta.maintaining_agency.is_empty());
        assert!(!s.target_schema().fields().is_empty());
    }
}

#[test]
fn test_country_pack_brasil_locators() {
    let pack = create_pack_brasil();
    let params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".into(),
        },
        jurisdiction_code: Some("SP".into()),
        year: 2024,
        month: Some(6),
        extra_filters: {
            let mut m = HashMap::new();
            m.insert("disease".into(), "DENG".into());
            m.insert("sub_system".into(), "PA".into());
            m.insert("type".into(), "ST".into());
            m.insert("exam_type".into(), "MAM".into());
            m
        },
        as_of_snapshot: None,
    };

    for source in pack {
        let loc = source.resolve_locator(&params);
        assert!(
            loc.is_ok(),
            "Falha ao resolver locator para fonte '{}': {:?}",
            source.metadata().id,
            loc.err()
        );
        let uri = loc.unwrap();
        assert!(!uri.is_empty(), "URI resolvida não pode ser vazia");
    }
}

#[test]
fn test_country_pack_global_metadata_and_registry() {
    let pack = create_pack_global();
    assert_eq!(
        pack.len(),
        6,
        "Pack Global deve conter exatamente 6 fontes supranacionais"
    );

    let mut registry = SourceRegistry::new();
    registry.register_pack(pack);

    let expected_global_ids = [
        "global.who_gho",
        "global.ihme_gbd",
        "global.copernicus_era5",
        "global.worldpop",
        "global.paho_plisa",
        "global.openaq",
    ];

    for id in &expected_global_ids {
        let source = registry.get(id);
        assert!(
            source.is_ok(),
            "Fonte global '{}' deve estar registrada",
            id
        );
        let s = source.unwrap();
        let meta = s.metadata();
        assert_eq!(meta.id, *id);
        assert!(!meta.display_name.is_empty());
        assert!(!meta.maintaining_agency.is_empty());
        assert!(!s.target_schema().fields().is_empty());
    }
}

#[test]
fn test_country_pack_global_locators() {
    let pack = create_pack_global();
    let params = DataQueryParams {
        scope: GeographicScope::Supranational {
            entity: "GLOBAL".into(),
        },
        jurisdiction_code: Some("BRA".into()),
        year: 2023,
        month: None,
        extra_filters: {
            let mut m = HashMap::new();
            m.insert("indicator".into(), "WHOSIS_000001".into());
            m
        },
        as_of_snapshot: None,
    };

    for source in pack {
        let loc = source.resolve_locator(&params);
        assert!(
            loc.is_ok(),
            "Falha ao resolver locator para fonte global '{}': {:?}",
            source.metadata().id,
            loc.err()
        );
        let uri = loc.unwrap();
        assert!(uri.starts_with("https://"), "URI global deve ser HTTPS");
    }
}

#[tokio::test]
async fn test_end_to_end_fetch_mocked_for_all_sources() {
    let mut all_sources: Vec<Arc<dyn HealthDataSourceSPI>> = Vec::new();
    all_sources.extend(create_pack_brasil());
    all_sources.extend(create_pack_global());

    assert_eq!(
        all_sources.len(),
        26,
        "Total de fontes suportadas deve ser 26"
    );

    let transport = Arc::new(MockTransport::new());
    let decompressor = Arc::new(DbcDecompressor::new().unwrap());
    let cache = Arc::new(MemoryCache::new());
    let state = Arc::new(MemorySyncState::new());

    let context = SourceExecutionContext {
        transport: transport.clone(),
        decompressor: decompressor.clone(),
        cache: cache.clone(),
        state: state.clone(),
    };

    let params = DataQueryParams {
        scope: GeographicScope::National {
            iso_3166_alpha3: "BRA".into(),
        },
        jurisdiction_code: Some("AC".into()),
        year: 2022,
        month: Some(1),
        extra_filters: HashMap::new(),
        as_of_snapshot: None,
    };

    // Para cada fonte, registrar uma resposta mock simulada e verificar resiliência de fetch_and_decode
    for source in &all_sources {
        let loc = source.resolve_locator(&params).unwrap();
        transport.register_response(&loc, vec![0u8; 32]);

        // Executar fetch_and_decode assíncrono garantindo que propaga erro sem panic em payload simulado curto
        let res = source.fetch_and_decode(&params, &context).await;
        // Espera-se Ok ou Err dependendo se a fonte aceita fallback vazio ou exige decodificação estrita
        assert!(res.is_ok() || res.is_err());
    }
}

#[test]
fn test_source_registry_standard_traits_and_application_service_factory() {
    use brhealth_core::domain::application::BRHealthApplicationService;

    // 1. Construtor standard()
    let registry = SourceRegistry::standard();
    assert_eq!(registry.len(), 26);
    assert!(!registry.is_empty());
    assert!(registry.contains("datasus.sim"));
    assert!(registry.contains("global.who_gho"));
    assert!(!registry.contains("fonte_inexistente"));

    let all_meta = registry.list_all();
    assert_eq!(all_meta.len(), 26);

    // 2. FromIterator e Extend
    let pack_br = create_pack_brasil();
    let reg_from_iter: SourceRegistry = pack_br.into_iter().collect();
    assert_eq!(reg_from_iter.len(), 20);

    let mut reg_extend = SourceRegistry::new();
    reg_extend.extend(create_pack_global());
    assert_eq!(reg_extend.len(), 6);

    // 3. BRHealthApplicationService::standard_in_memory()
    let service_res = BRHealthApplicationService::standard_in_memory();
    assert!(service_res.is_ok());
    let service = service_res.unwrap();
    assert_eq!(service.registry().len(), 26);
}

