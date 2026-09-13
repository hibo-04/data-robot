use std::sync::Arc;

use analytics::pipeline::query_from_analysis;
use analytics::query::SemanticQuery;
use analytics::reports::Visualization;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;

use crate::error::WebError;
use crate::html::{
    self, chart_from_result, scalar_from_result, Dashboard, ExploreView, KpiValue, SourceView,
};
use crate::overlay::{publish, queryable, Overlay, ReviewStatus};
use crate::state::{pack_list, parse_packs, AppState};

#[derive(Deserialize)]
struct PacksQuery {
    packs: Option<String>,
}

#[derive(Deserialize)]
struct AnalyzeForm {
    fixture: String,
    #[serde(default)]
    packs: String,
}

#[derive(Deserialize)]
struct OverlayForm {
    kind: String,
    status: String,
    #[serde(default)]
    from: String,
    #[serde(default)]
    to: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    packs: String,
}

#[derive(Deserialize)]
struct QueryForm {
    measure: String,
    #[serde(default)]
    dimension: String,
    #[serde(default)]
    packs: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/style.css", get(style))
        .route("/analyze", post(analyze))
        .route("/source/{name}", get(source_page))
        .route("/source/{name}/overlay", post(overlay))
        .route("/source/{name}/query", post(run_query))
        .with_state(Arc::new(state))
}

async fn style() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("style.css"),
    )
}

async fn home(State(state): State<Arc<AppState>>) -> Html<String> {
    Html(html::home_page(
        &state.list_fixtures(),
        analytics::dictionary::DEFAULT_PACKS,
        None,
    ))
}

async fn analyze(Form(form): Form<AnalyzeForm>) -> Result<Redirect, WebError> {
    let packs = parse_packs(&form.packs);
    if !crate::state::fixture_id_ok(&form.fixture) {
        return Err(WebError::bad("invalid fixture name"));
    }
    Ok(Redirect::to(&format!(
        "/source/{}?packs={}",
        form.fixture, packs
    )))
}

async fn source_page(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(q): Query<PacksQuery>,
) -> Result<Html<String>, WebError> {
    let packs = parse_packs(q.packs.as_deref().unwrap_or_default());
    let view = build_source(&state, &name, &packs, None, None)?;
    Ok(Html(html::source_page(&view)))
}

async fn overlay(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(form): Form<OverlayForm>,
) -> Result<Response, WebError> {
    let packs = parse_packs(&form.packs);
    let pack_names = pack_list(&packs);
    let status =
        ReviewStatus::parse(&form.status).ok_or_else(|| WebError::bad("unknown review status"))?;
    let analysis = state.analyze(&name, &packs)?;
    let mut overlay = state.load_overlay(&name, &pack_names);
    overlay.packs = pack_names.clone();

    match form.kind.as_str() {
        "relationship" => overlay.set_relationship(&form.from, &form.to, status),
        "kpi" => {
            let label = if form.label.trim().is_empty() {
                None
            } else {
                Some(form.label.clone())
            };
            overlay.set_kpi(&form.id, status, label);
        }
        "identity" => overlay.set_identity(&form.id, status),
        _ => return Err(WebError::bad("unknown overlay kind")),
    }
    state.save_overlay(&overlay)?;

    if is_htmx(&headers) {
        let fragment = match form.kind.as_str() {
            "relationship" => analysis
                .relationships
                .relationships
                .iter()
                .find(|r| r.from.qualified() == form.from && r.to.qualified() == form.to)
                .map(|r| html::relationship_card(&name, &packs, r, overlay.relationship_status(r)))
                .ok_or_else(|| WebError::not_found("relationship not found"))?,
            "kpi" => analysis
                .kpis
                .iter()
                .find(|k| k.id == form.id)
                .map(|k| html::kpi_review_card(&name, &packs, k, &overlay))
                .ok_or_else(|| WebError::not_found("KPI not found"))?,
            "identity" => analysis
                .identities
                .iter()
                .find(|i| i.id == form.id)
                .map(|i| {
                    html::identity_review_card(&name, &packs, i, overlay.identity_status(&i.id))
                })
                .ok_or_else(|| WebError::not_found("identity not found"))?,
            _ => unreachable!(),
        };
        Ok(Html(fragment).into_response())
    } else {
        Ok(Redirect::to(&format!("/source/{name}?packs={packs}")).into_response())
    }
}

async fn run_query(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Form(form): Form<QueryForm>,
) -> Result<Response, WebError> {
    let packs = parse_packs(&form.packs);
    let pack_names = pack_list(&packs);
    let analysis = state.analyze(&name, &packs)?;
    let overlay = state.load_overlay(&name, &pack_names);
    let published = publish(&analysis, &overlay);
    let connector = state.connector(&name)?;

    let mut dimensions = Vec::new();
    if !form.dimension.trim().is_empty() {
        dimensions.push(form.dimension.trim().to_string());
    }
    let query = SemanticQuery {
        measures: vec![form.measure.clone()],
        dimensions,
        filters: Vec::new(),
        order_by: None,
        limit: Some(50),
    };
    let (_plan, sql, result) =
        query_from_analysis(&connector, &published, &query).map_err(WebError::from_analytics)?;
    let explore = ExploreView {
        chart: chart_from_result(&result),
        sql,
        result,
    };

    if is_htmx(&headers) {
        Ok(Html(html::explore_result(&explore)).into_response())
    } else {
        let view = build_source(&state, &name, &packs, Some(explore), None)?;
        Ok(Html(html::source_page(&view)).into_response())
    }
}

fn build_source(
    state: &AppState,
    name: &str,
    packs: &str,
    explore: Option<ExploreView>,
    error: Option<String>,
) -> Result<SourceView, WebError> {
    let analysis = state.analyze(name, packs)?;
    let overlay = state.load_overlay(name, &pack_list(packs));
    let dashboard = build_dashboard(state, name, &analysis, &overlay)?;
    Ok(SourceView {
        name: name.to_string(),
        packs: packs.to_string(),
        analysis,
        overlay,
        dashboard,
        explore,
        error,
    })
}

fn build_dashboard(
    state: &AppState,
    name: &str,
    analysis: &analytics::pipeline::Analysis,
    overlay: &Overlay,
) -> Result<Dashboard, WebError> {
    let published = publish(analysis, overlay);
    let connector = state.connector(name)?;
    let mut cards = Vec::new();
    for kpi in published.kpis.iter().filter(|k| queryable(k)).take(5) {
        let query = SemanticQuery {
            measures: vec![kpi.display_name().to_string()],
            dimensions: Vec::new(),
            filters: Vec::new(),
            order_by: None,
            limit: Some(1),
        };
        if let Ok((_plan, _sql, result)) = query_from_analysis(&connector, &published, &query) {
            if let Some(value) = scalar_from_result(&result) {
                cards.push(KpiValue {
                    title: kpi.display_name().to_string(),
                    value,
                });
            }
        }
    }

    let mut chart_title = None;
    let mut chart = None;
    let mut table = None;
    let mut sql = None;
    let chart_report = published.reports.iter().find(|r| {
        matches!(
            r.visualization,
            Visualization::BarChart | Visualization::LineChart
        ) && r.measure.is_some()
            && r.dimension.is_some()
    });
    if let Some(report) = chart_report {
        let query = SemanticQuery {
            measures: vec![report.measure.clone().unwrap()],
            dimensions: vec![report.dimension.clone().unwrap()],
            filters: Vec::new(),
            order_by: None,
            limit: Some(24),
        };
        if let Ok((_plan, qsql, result)) = query_from_analysis(&connector, &published, &query) {
            chart_title = Some(report.title.clone());
            chart = chart_from_result(&result);
            sql = Some(qsql);
            table = Some(result);
        }
    }

    Ok(Dashboard {
        cards,
        chart_title,
        chart,
        table,
        sql,
    })
}

fn is_htmx(headers: &HeaderMap) -> bool {
    headers
        .get("hx-request")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "true")
}
