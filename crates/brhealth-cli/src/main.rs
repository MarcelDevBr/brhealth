// Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.
// Licensed under the GNU Affero General Public License v3 (AGPLv3)
// or a commercial license agreement directly with the author.

//! Interface de Linha de Comando (CLI) nativa de alta performance para o motor BRHealth.
//!
//! Fornece ferramentas analíticas para consulta de fontes em saúde pública,
//! validação territorial do IBGE, classificação de CSAP e auditoria FAIR/PROV-O.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

use brhealth_core::decoders::dbc::DbcDecompressor;
use brhealth_core::domain::analytics::csap::{
    classify_cid10, compute_csap_metrics, compute_primary_care_roi,
};
use brhealth_core::domain::analytics::mortality::{compute_apvp, compute_apvp_rate};
use brhealth_core::domain::application::{
    BRHealthApplicationService, PipelineExecutionOptions,
};
use brhealth_core::domain::source_spi::{
    DataQueryParams, GeographicScope, SourceExecutionContext,
};
use brhealth_core::domain::spatial::s2::{
    coord_to_s2_cell, s2_cell_to_coord, DEFAULT_S2_MUNICIPAL_LEVEL,
};
use brhealth_core::domain::transforms::ibge::{calculate_ibge_dv, harmonize_ibge_code};
use brhealth_core::domain::transforms::ontology::MedicalOntologyHarmonizer;
use brhealth_core::infrastructure::cache::MemoryCache;
use brhealth_core::infrastructure::state::MemorySyncState;
use brhealth_core::infrastructure::transport::AsyncFtpTransport;
use brhealth_core::sources::{create_pack_brasil, create_pack_global};
use brhealth_core::SourceRegistry;

#[derive(Parser)]
#[command(
    name = "brhealth",
    author = "Marcel <MarcelDevBr>",
    version,
    about = "BRHealth - Motor Analítico Colunar de Alta Performance para Saúde Coletiva",
    long_about = "Motor colunar em Apache Arrow para processamento analítico de dados do SUS, IBGE e determinantes sociais."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Exibe a versão e os metadados de licenciamento e copyright do BRHealth.
    Version,

    /// Lista todas as fontes de dados de saúde e determinantes sociais registradas.
    Sources,

    /// Calcula e valida o Dígito Verificador (DV) do IBGE pelo algoritmo de Luhn Módulo 10.
    Dv {
        /// Código municipal do IBGE com 6 dígitos (ex: 355030 para São Paulo).
        code: String,
    },

    /// Classifica um código CID-10 conforme os 19 grupos de CSAP (Portaria MS/SAS nº 221/2008).
    Csap {
        /// Código CID-10 a ser classificado (ex: J45, I10, E10).
        cid: String,
    },

    /// Calcula o Retorno sobre Investimento (ROI) em Saúde Coletiva na Atenção Primária.
    Roi {
        /// Custo hospitalar direto evitável em reais (R$).
        #[arg(long)]
        avoidable_cost: f64,

        /// Investimento orçamentário na Estratégia Saúde da Família em reais (R$).
        #[arg(long)]
        investment: f64,

        /// Fração de impacto atribuível (entre 0.0 e 1.0).
        #[arg(long, default_value_t = 0.50)]
        attributable_fraction: f64,
    },

    /// Executa pipeline analítico colunar completo (ingestão, H3, CSAP, FAIR W3C PROV-O).
    Fetch {
        /// Identificador da fonte de dados (ex: datasus_sim, datasus_sih, ibge_censo, copernicus_era5).
        #[arg(short, long)]
        source: String,

        /// Sigla da Unidade Federativa (ex: AC, SP, RJ) ou código de jurisdição.
        #[arg(short, long)]
        uf: Option<String>,

        /// Ano de competência dos dados (ex: 2022).
        #[arg(short, long, default_value_t = 2022)]
        year: u16,

        /// Mês de competência dos dados (opcional, 1 a 12).
        #[arg(short, long)]
        month: Option<u8>,

        /// Habilita harmonização de códigos municipais do IBGE para 7 dígitos canônicos.
        #[arg(long, default_value_t = true)]
        harmonize_ibge: bool,

        /// Resolução Uber H3 para indexação espacial discreta (0 a 15).
        #[arg(long)]
        h3_resolution: Option<u8>,

        /// Habilita enriquecimento de causas de internação sensíveis à atenção primária (CSAP).
        #[arg(long, default_value_t = false)]
        enrich_csap: bool,

        /// Caminho para exportação do resultado colunar em formato Apache Parquet.
        #[arg(short, long)]
        out_parquet: Option<PathBuf>,
    },

    /// Calcula os Anos Potenciais de Vida Perdidos (APVP / YLL) para idades de óbito prematuro.
    Apvp {
        /// Idades de óbito separadas por espaço (ex: 35 42 18 55).
        #[arg(required = true, num_args = 1..)]
        ages: Vec<u16>,

        /// Idade limite de corte de morte prematura (padrão: 70 anos).
        #[arg(short, long, default_value_t = 70)]
        cutoff: u16,

        /// População de referência para cálculo da taxa por 100.000 hab.
        #[arg(short, long)]
        population: Option<u64>,
    },

    /// Converte latitude e longitude em um identificador S2 CellId de 64 bits.
    S2 {
        /// Latitude geográfica em graus decimais (-90 a 90).
        #[arg(long, allow_hyphen_values = true)]
        lat: f64,

        /// Longitude geográfica em graus decimais (-180 a 180).
        #[arg(long, allow_hyphen_values = true)]
        lon: f64,

        /// Nível de resolução da célula S2 (0 a 30, padrão: 10 - nível municipal).
        #[arg(short, long, default_value_t = DEFAULT_S2_MUNICIPAL_LEVEL)]
        level: u8,
    },

    /// Mapeia código histórico da CID-9 para o equivalente canônico na CID-10.
    Cid9 {
        /// Código CID-9 (ex: 250, 401, 410, 493, E819).
        code: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("BRHealth v{}", env!("CARGO_PKG_VERSION"));
            println!("Copyright (c) 2024-2026 Marcel <MarcelDevBr> and BRHealth Contributors.");
            println!("Licença: GNU Affero General Public License v3 (AGPLv3) com opção comercial.");
            println!("Arquitetura: Hexagonal Data-Oriented Design (Hexagonal DOD) com Apache Arrow.");
        }

        Commands::Sources => {
            let br_pack = create_pack_brasil();
            let global_pack = create_pack_global();

            println!("=== Fontes Analíticas Registradas no BRHealth ===");
            println!("{:<20} | {:<42} | {:<16} | {:<12}", "ID", "NOME", "ÓRGÃO", "RESOLUÇÃO");
            println!("{:-<20}-|-{:-<42}-|-{:-<16}-|-{:-<12}", "", "", "", "");

            for source in br_pack.iter().chain(global_pack.iter()) {
                let meta = source.metadata();
                println!(
                    "{:<20} | {:<42} | {:<16} | {:<12}",
                    meta.id, meta.display_name, meta.maintaining_agency, meta.temporal_resolution
                );
            }
            println!("\nTotal de fontes disponíveis: {}", br_pack.len() + global_pack.len());
        }

        Commands::Dv { code } => {
            match calculate_ibge_dv(&code) {
                Ok(dv) => {
                    let harmonized = harmonize_ibge_code(&code)?;
                    println!("Código original:   {}", code);
                    println!("Dígito Verificador (DV): {}", dv);
                    println!("Código harmonizado (7 dígitos): {}", harmonized);
                }
                Err(err) => {
                    eprintln!("Erro ao calcular DV para o código '{}': {}", code, err);
                    std::process::exit(1);
                }
            }
        }

        Commands::Csap { cid } => {
            match classify_cid10(&cid) {
                Some(group) => {
                    println!("Código CID-10: {}", cid.to_uppercase());
                    println!("Classificação: CONDIÇÃO SENSÍVEL À ATENÇÃO PRIMÁRIA (CSAP)");
                    println!("Grupo {}: {}", group.id(), group.name());
                }
                None => {
                    println!("Código CID-10: {}", cid.to_uppercase());
                    println!("Classificação: NÃO-CSAP (Causa geral ou não prevenível na APS)");
                }
            }
        }

        Commands::Roi {
            avoidable_cost,
            investment,
            attributable_fraction,
        } => {
            match compute_primary_care_roi(avoidable_cost, investment, attributable_fraction) {
                Ok(roi) => {
                    let economizado = avoidable_cost * attributable_fraction;
                    println!("=== Avaliação Econômica de Atenção Primária à Saúde ===");
                    println!("Custo Hospitalar Evitável:    R$ {:>12.2}", avoidable_cost);
                    println!("Investimento na ESF/APS:       R$ {:>12.2}", investment);
                    println!("Fração Atribuível:             {:>12.1}%", attributable_fraction * 100.0);
                    println!("Economia Líquida Estimada:     R$ {:>12.2}", economizado);
                    println!("ROI (Retorno sobre Investimento): {:>9.2}%", roi * 100.0);
                    if roi > 0.0 {
                        println!("Status: ECONÔMICAMENTE SUPERAVITÁRIO (Gera valor líquido ao erário)");
                    } else {
                        println!("Status: NECESSITA DE AJUSTE DE EFICIÊNCIA OU MAIOR ATRIBUIÇÃO");
                    }
                }
                Err(err) => {
                    eprintln!("Erro no cálculo de ROI: {}", err);
                    std::process::exit(1);
                }
            }
        }

        Commands::Fetch {
            source,
            uf,
            year,
            month,
            harmonize_ibge,
            h3_resolution,
            enrich_csap,
            out_parquet,
        } => {
            println!("Iniciando pipeline analítico BRHealth para fonte '{}'...", source);

            let mut registry = SourceRegistry::new();
            registry.register_pack(create_pack_brasil());
            registry.register_pack(create_pack_global());
            let registry = Arc::new(registry);

            let ftp_transport = Arc::new(AsyncFtpTransport::new_datasus());
            let decompressor = Arc::new(DbcDecompressor::new()?);
            let sync_state = Arc::new(MemorySyncState::new());

            let context = Arc::new(SourceExecutionContext {
                transport: ftp_transport,
                decompressor,
                cache: Arc::new(MemoryCache::new()),
                state: sync_state.clone(),
            });

            let app_service = Arc::new(BRHealthApplicationService::new(
                registry.clone(),
                context,
                sync_state,
            ));

            let params = DataQueryParams {
                scope: GeographicScope::National {
                    iso_3166_alpha3: "BRA".into(),
                },
                jurisdiction_code: uf,
                year,
                month,
                extra_filters: HashMap::new(),
                as_of_snapshot: None,
            };

            let options = PipelineExecutionOptions {
                harmonize_ibge,
                assign_h3_resolution: h3_resolution,
                h3_coord_columns: None,
                enrich_csap,
                reference_population: None,
                persist_to_cache: false,
                cache_base_path: None,
            };

            let result = app_service
                .execute_full_pipeline(&source, &params, &options)
                .await?;

            let total_rows: usize = result.batches.iter().map(|b| b.num_rows()).sum();
            println!("\n=== Execução Concluída com Sucesso ===");
            println!("Total de RecordBatches gerados: {}", result.batches.len());
            println!("Total de registros (linhas):    {}", total_rows);

            if let Some(first_batch) = result.batches.first() {
                println!("Total de colunas Arrow:         {}", first_batch.num_columns());
                println!("Esquema Arrow gerado:");
                for field in first_batch.schema().fields() {
                    println!("  - {}: {:?}", field.name(), field.data_type());
                }

                if enrich_csap
                    && let Ok(csap_metrics) = compute_csap_metrics(first_batch, None)
                {
                    println!("\n=== Métricas Epidemiológicas CSAP ===");
                    println!("Total de Internações:          {}", csap_metrics.total_admissions);
                    println!("Internações Evitáveis (CSAP):  {}", csap_metrics.csap_admissions);
                    println!("Proporção CSAP:                {:.2}%", csap_metrics.csap_proportion * 100.0);
                }
            }

            println!("\n=== Manifesto de Linhagem Científica FAIR (W3C PROV-O) ===");
            println!("Run UUID:          {}", result.manifest.execution_metadata.run_uuid);
            println!("Timestamp UTC:     {}", result.manifest.execution_metadata.timestamp_utc);
            for src in &result.manifest.sources {
                println!("Fonte Primária:    {} ({})", src.source_name, src.scope);
                println!("  URI:             {}", src.uri);
                println!("  SHA-256 Bruto:   {}", src.sha256_raw_payload);
            }
            println!("Etapas executadas: {}", result.manifest.pipeline_steps.len());

            if let Some(target_path) = out_parquet {
                if let Some(batch) = result.batches.first() {
                    let file = File::create(&target_path)?;
                    let mut writer = parquet::arrow::arrow_writer::ArrowWriter::try_new(
                        file,
                        batch.schema(),
                        None,
                    )?;
                    for b in &result.batches {
                        writer.write(b)?;
                    }
                    writer.close()?;
                    println!("\nDados salvos com sucesso em Parquet: {:?}", target_path);
                } else {
                    println!("\nNenhum dado retornado para salvar em Parquet.");
                }
            }
        }

        Commands::Apvp {
            ages,
            cutoff,
            population,
        } => {
            let total_apvp = compute_apvp(&ages, cutoff);
            println!("=== Indicadores de Mortalidade Prematura (APVP / YLL) ===");
            println!("Total de óbitos analisados: {}", ages.len());
            println!("Idade limite de corte:      {} anos", cutoff);
            println!("Total de APVP acumulado:    {} anos de vida perdidos", total_apvp);

            if let Some(pop) = population {
                match compute_apvp_rate(total_apvp, pop) {
                    Ok(rate) => {
                        println!("População sob o corte:      {}", pop);
                        println!("Taxa de APVP padronizada:   {:.2} por 100.000 hab.", rate);
                    }
                    Err(err) => {
                        eprintln!("Erro ao calcular taxa de APVP: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        }

        Commands::S2 { lat, lon, level } => {
            match coord_to_s2_cell(lat, lon, level) {
                Ok(cell_id) => {
                    let (center_lat, center_lon) = s2_cell_to_coord(cell_id)?;
                    println!("=== Indexação Espacial Esférica S2 Geometry ===");
                    println!("Coordenadas de entrada: lat={:.6}, lon={:.6}", lat, lon);
                    println!("Nível de resolução:     {}", level);
                    println!("S2 CellID (decimal):    {}", cell_id);
                    println!("S2 CellID (hexadecimal):0x{:016x}", cell_id);
                    println!("Centro da célula S2:    lat={:.6}, lon={:.6}", center_lat, center_lon);
                }
                Err(err) => {
                    eprintln!("Erro ao calcular célula S2: {}", err);
                    std::process::exit(1);
                }
            }
        }

        Commands::Cid9 { code } => {
            let harmonizer = MedicalOntologyHarmonizer::new();
            match harmonizer.map_icd9_to_icd10(&code) {
                Some(icd10) => {
                    println!("Código CID-9 original:  {}", code.to_uppercase());
                    println!("Código CID-10 mapeado:  {}", icd10);
                }
                None => {
                    eprintln!("Código CID-9 '{}' não possui mapeamento direto cadastrado.", code);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
