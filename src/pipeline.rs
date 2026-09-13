use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::connector::DataSource;
use crate::dictionary::{parse_pack_list, DictionarySet, DEFAULT_PACKS};
use crate::error::Result;
use crate::identities::{detect_identities, ColumnIdentity};
use crate::kpi::{detect_kpis, KpiCandidate};
use crate::profiling::{build_profiles, DatabaseProfile};
use crate::query::{plan_query, QueryPlan, QueryResult, SemanticQuery};
use crate::relationships::{detect_relationships, RelationshipSet};
use crate::reports::{suggest_reports, ReportSuggestion};
use crate::schema::DatabaseSchema;
use crate::semantic::{build_semantic_model, SemanticModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTimings {
    pub schema_ms: u128,
    pub profile_ms: u128,
    pub relationships_ms: u128,
    pub identities_ms: u128,
    pub semantic_ms: u128,
    pub kpis_ms: u128,
    pub reports_ms: u128,
    pub total_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub source_name: String,
    pub dictionary_packs: Vec<String>,
    pub schema: DatabaseSchema,
    pub profiles: DatabaseProfile,
    pub relationships: RelationshipSet,
    pub identities: Vec<ColumnIdentity>,
    pub semantic_model: SemanticModel,
    pub kpis: Vec<KpiCandidate>,
    pub reports: Vec<ReportSuggestion>,
    pub timings: PipelineTimings,
}

pub fn analyze<S: DataSource>(source: &S) -> Result<Analysis> {
    analyze_with_packs(source, DEFAULT_PACKS)
}

pub fn analyze_with_packs<S: DataSource>(source: &S, packs: &str) -> Result<Analysis> {
    let started = Instant::now();

    let t0 = Instant::now();
    let schema = source.schema()?;
    let schema_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let raw = source.profile_raw()?;
    let profiles = build_profiles(&schema, &raw);
    let profile_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let relationships = detect_relationships(source, &schema, &profiles)?;
    let relationships_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let identities = detect_identities(source, &schema, &profiles, &relationships)?;
    let identities_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let semantic_model = build_semantic_model(source.name(), &schema, &profiles, &relationships);
    let semantic_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let pack_names = parse_pack_list(packs);
    let dict = DictionarySet::bundled()?.with_packs(
        &pack_names.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut kpis = detect_kpis(&semantic_model);
    dict.apply(&semantic_model, &mut kpis);
    crate::kpi::drop_double_counted_kpis(&mut kpis, &identities);
    let derived = dict.derived_kpis(&kpis);
    kpis.extend(derived);
    kpis = crate::kpi::dedup_kpis(kpis);
    let kpis_ms = t0.elapsed().as_millis();

    let t0 = Instant::now();
    let reports = suggest_reports(&semantic_model, &profiles, &kpis);
    let reports_ms = t0.elapsed().as_millis();

    Ok(Analysis {
        source_name: source.name().to_string(),
        dictionary_packs: dict.pack_names(),
        schema,
        profiles,
        relationships,
        identities,
        semantic_model,
        kpis,
        reports,
        timings: PipelineTimings {
            schema_ms,
            profile_ms,
            relationships_ms,
            identities_ms,
            semantic_ms,
            kpis_ms,
            reports_ms,
            total_ms: started.elapsed().as_millis(),
        },
    })
}

pub fn query_from_analysis(
    source: &impl DataSource,
    analysis: &Analysis,
    query: &SemanticQuery,
) -> Result<(QueryPlan, String, QueryResult)> {
    let plan = plan_query(&analysis.semantic_model, &analysis.kpis, query)?;
    let sql = plan.to_sql(source.dialect())?;
    let result = source.execute_plan(&plan)?;
    Ok((plan, sql, result))
}
