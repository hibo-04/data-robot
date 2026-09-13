use std::fs;
use std::path::{Path, PathBuf};

use analytics::benchmark::{evaluate, load_ground_truth};
use analytics::connector::{FixtureConnector, PostgresConnector};
use analytics::identities::IdentityScope;
use analytics::pipeline::{analyze_with_packs, query_from_analysis, Analysis};
use analytics::query::SemanticQuery;
use analytics::relationships::RelationshipBand;
use analytics::dictionary::DEFAULT_PACKS;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "analytics", about = "Local analytics-core proof of concept")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover schema, profile data, infer relationships, KPIs and reports
    Inspect {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        json: bool,
    },
    /// Write semantic-model.json, relationships.json, identities.json, kpis.json, reports.json
    ExportModel {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long, default_value = "./out")]
        out: PathBuf,
    },
    DetectRelationships {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        json: bool,
    },
    DetectKpis {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        json: bool,
    },
    DetectIdentities {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        json: bool,
    },
    GenerateReports {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        json: bool,
    },
    GenerateQuery {
        #[command(flatten)]
        source: SourceArgs,
        #[arg(long)]
        query_json: PathBuf,
        #[arg(long)]
        execute: bool,
    },
    Benchmark {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long, default_value = DEFAULT_PACKS)]
        packs: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Args, Clone)]
struct SourceArgs {
    #[arg(long)]
    fixture: Option<PathBuf>,
    #[arg(long)]
    database_url: Option<String>,
    /// Comma-separated dictionary packs. Core KPIs work with none; default adds labels.
    #[arg(long, default_value = DEFAULT_PACKS)]
    packs: String,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> analytics::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Inspect { source, json } => {
            let analysis = analyze_source(&source).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            } else {
                print_inspect(&analysis);
            }
        }
        Commands::ExportModel { source, out } => {
            let analysis = analyze_source(&source).await?;
            export_model(&analysis, &out)?;
            println!("Wrote model files to {}", out.display());
        }
        Commands::DetectRelationships { source, json } => {
            let analysis = analyze_source(&source).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis.relationships)?);
            } else {
                print_relationships(&analysis);
            }
        }
        Commands::DetectKpis { source, json } => {
            let analysis = analyze_source(&source).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis.kpis)?);
            } else {
                print_kpis(&analysis);
            }
        }
        Commands::DetectIdentities { source, json } => {
            let analysis = analyze_source(&source).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis.identities)?);
            } else {
                print_identities(&analysis);
            }
        }
        Commands::GenerateReports { source, json } => {
            let analysis = analyze_source(&source).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis.reports)?);
            } else {
                print_reports(&analysis);
            }
        }
        Commands::GenerateQuery {
            source,
            query_json,
            execute,
        } => {
            let query: SemanticQuery = serde_json::from_str(&fs::read_to_string(&query_json)?)?;
            match &source.fixture {
                Some(path) => {
                    let connector = FixtureConnector::open(path)?;
                    let analysis = analyze_with_packs(&connector, &source.packs)?;
                    let (plan, sql, result) = query_from_analysis(&connector, &analysis, &query)?;
                    println!("{sql}");
                    if execute {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                    let _ = plan;
                }
                None => {
                    let url = source.database_url.as_deref().ok_or_else(|| {
                        analytics::AnalyticsError::msg("provide --fixture or --database-url")
                    })?;
                    let connector = PostgresConnector::connect(url).await?;
                    let analysis = analyze_with_packs(&connector, &source.packs)?;
                    let (_plan, sql, result) = query_from_analysis(&connector, &analysis, &query)?;
                    println!("{sql}");
                    if execute {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
            }
        }
        Commands::Benchmark { fixture, packs, json } => {
            let connector = FixtureConnector::open(&fixture)?;
            let analysis = analyze_with_packs(&connector, &packs)?;
            let Some(truth) = load_ground_truth(&fixture)? else {
                return Err(analytics::AnalyticsError::msg(
                    "fixture has no ground_truth.json (the core never reads this file during analysis)",
                ));
            };
            let report = evaluate(&analysis, &truth);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Benchmark against ground truth for {}", report.fixture);
                println!(
                    "Relationships  precision {:.3}  recall {:.3}  TP {}  FP {}  FN {}",
                    report.relationships.precision,
                    report.relationships.recall,
                    report.relationships.true_positives,
                    report.relationships.false_positives,
                    report.relationships.false_negatives
                );
                println!(
                    "KPIs           precision {:.3}  recall {:.3}  TP {}  FP {}  FN {}",
                    report.kpis.precision,
                    report.kpis.recall,
                    report.kpis.true_positives,
                    report.kpis.false_positives,
                    report.kpis.false_negatives
                );
                if !report.relationship_false_negatives.is_empty() {
                    println!("missed relationships: {}", report.relationship_false_negatives.join(", "));
                }
                if !report.relationship_false_positives.is_empty() {
                    println!("extra relationships: {}", report.relationship_false_positives.join(", "));
                }
                if !report.kpi_false_negatives.is_empty() {
                    println!("missed KPIs: {}", report.kpi_false_negatives.join(", "));
                }
                if !report.kpi_false_positives.is_empty() {
                    println!("extra KPIs: {}", report.kpi_false_positives.join(", "));
                }
            }
        }
    }
    Ok(())
}

async fn analyze_source(source: &SourceArgs) -> analytics::Result<Analysis> {
    if source.fixture.is_some() && source.database_url.is_some() {
        return Err(analytics::AnalyticsError::msg(
            "use either --fixture or --database-url, not both",
        ));
    }
    if let Some(path) = &source.fixture {
        let connector = FixtureConnector::open(path)?;
        return analyze_with_packs(&connector, &source.packs);
    }
    if let Some(url) = &source.database_url {
        let connector = PostgresConnector::connect(url).await?;
        return analyze_with_packs(&connector, &source.packs);
    }
    Err(analytics::AnalyticsError::msg(
        "provide --fixture <dir> or --database-url <postgres url>",
    ))
}

fn export_model(analysis: &Analysis, out: &Path) -> analytics::Result<()> {
    fs::create_dir_all(out)?;
    fs::write(
        out.join("semantic-model.json"),
        serde_json::to_string_pretty(&analysis.semantic_model)?,
    )?;
    fs::write(
        out.join("relationships.json"),
        serde_json::to_string_pretty(&analysis.relationships)?,
    )?;
    fs::write(
        out.join("identities.json"),
        serde_json::to_string_pretty(&analysis.identities)?,
    )?;
    fs::write(
        out.join("kpis.json"),
        serde_json::to_string_pretty(&analysis.kpis)?,
    )?;
    fs::write(
        out.join("reports.json"),
        serde_json::to_string_pretty(&analysis.reports)?,
    )?;
    Ok(())
}

fn print_inspect(analysis: &Analysis) {
    println!("Data source analyzed.\n");
    if !analysis.dictionary_packs.is_empty() {
        println!("Dictionary packs: {}", analysis.dictionary_packs.join(", "));
    }
    println!("Tables discovered: {}", analysis.schema.tables.len());
    println!("Columns discovered: {}", analysis.schema.column_count());
    println!("\nProfiling complete.\n");
    print_relationships(analysis);
    println!();
    print_identities(analysis);
    println!();
    print_kpis(analysis);
    println!();
    print_reports(analysis);
    println!(
        "\nElapsed: schema {}ms, profile {}ms, relationships {}ms, identities {}ms, semantic {}ms, kpis {}ms, reports {}ms (total {}ms)",
        analysis.timings.schema_ms,
        analysis.timings.profile_ms,
        analysis.timings.relationships_ms,
        analysis.timings.identities_ms,
        analysis.timings.semantic_ms,
        analysis.timings.kpis_ms,
        analysis.timings.reports_ms,
        analysis.timings.total_ms
    );
}

fn print_relationships(analysis: &Analysis) {
    let (declared, inferred, uncertain) = analysis.relationships.counts();
    println!("Relationships:");
    println!("Declared: {declared}");
    println!("Inferred: {inferred}");
    println!("Uncertain: {uncertain}");
    for rel in &analysis.relationships.relationships {
        let tag = match rel.band {
            RelationshipBand::Declared => "declared",
            RelationshipBand::Inferred => "inferred",
            RelationshipBand::Uncertain => "uncertain",
        };
        println!(
            "  [{tag} {:.0}%] {} → {} ({:?})",
            rel.confidence * 100.0,
            rel.from,
            rel.to,
            rel.relationship_type
        );
    }
}

fn print_identities(analysis: &Analysis) {
    println!("Numeric identities (name-free):");
    if analysis.identities.is_empty() {
        println!("  (none)");
        return;
    }
    for id in &analysis.identities {
        let scope = match id.scope {
            IdentityScope::IntraTable => "table",
            IdentityScope::Join => "join",
            IdentityScope::Grain => "grain",
        };
        println!(
            "  [{scope} {:.0}%] {}  ({:?}, {:.1}% rows)",
            id.confidence * 100.0,
            id.expression,
            id.certainty,
            id.match_ratio * 100.0
        );
    }
}

fn print_kpis(analysis: &Analysis) {
    println!("KPI candidates:");
    for kpi in &analysis.kpis {
        let label = kpi.display_name();
        if kpi.business_label.is_some() && kpi.business_label.as_deref() != Some(kpi.name.as_str()) {
            println!(
                "  {:<24} {:>3.0}%  ({:?})  [{} → {}]",
                label,
                kpi.confidence * 100.0,
                kpi.certainty,
                kpi.dictionary_pack.as_deref().unwrap_or("-"),
                kpi.name
            );
        } else {
            println!(
                "  {:<24} {:>3.0}%  ({:?})",
                label,
                kpi.confidence * 100.0,
                kpi.certainty
            );
        }
    }
}

fn print_reports(analysis: &Analysis) {
    println!("Suggested reports:");
    for report in &analysis.reports {
        println!("  {} ({:?})", report.title, report.visualization);
    }
}
