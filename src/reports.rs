use serde::{Deserialize, Serialize};

use crate::kpi::KpiCandidate;
use crate::profiling::DatabaseProfile;
use crate::semantic::{SemanticModel, SemanticRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visualization {
    KpiCard,
    LineChart,
    BarChart,
    PieChart,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSuggestion {
    pub title: String,
    pub visualization: Visualization,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_granularity: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

pub fn suggest_reports(
    model: &SemanticModel,
    profiles: &DatabaseProfile,
    kpis: &[KpiCandidate],
) -> Vec<ReportSuggestion> {
    let cards: Vec<&KpiCandidate> = kpis
        .iter()
        .filter(|k| k.source.is_some() && k.aggregation.is_some() && k.confidence >= 0.75)
        .filter(|k| matches!(k.aggregation, Some(crate::kpi::Aggregation::Sum)) || k.business_label.is_some())
        .take(8)
        .collect();
    let base_kpis: Vec<&KpiCandidate> = if cards.is_empty() {
        kpis.iter()
            .filter(|k| k.source.is_some() && k.aggregation.is_some() && k.confidence >= 0.75)
            .collect()
    } else {
        cards
    };
    let sum_kpis: Vec<&KpiCandidate> = base_kpis
        .iter()
        .copied()
        .filter(|k| matches!(k.aggregation, Some(crate::kpi::Aggregation::Sum)))
        .collect();
    let chart_kpis = if sum_kpis.is_empty() {
        base_kpis.iter().copied().take(3).collect::<Vec<_>>()
    } else {
        sum_kpis.iter().copied().take(3).collect()
    };

    let mut reports = Vec::new();
    for kpi in base_kpis.iter().take(5) {
        reports.push(ReportSuggestion {
            title: kpi.display_name().to_string(),
            visualization: Visualization::KpiCard,
            measure: Some(kpi.display_name().to_string()),
            dimension: None,
            time_granularity: None,
            confidence: kpi.confidence,
            reason: format!(
                "single measure `{}` is shown as a KPI card. {}",
                kpi.display_name(), kpi.reason
            ),
        });
    }

    let time_dims = related_time_dimensions(model, profiles, &base_kpis);
    if let Some((dim, gran)) = time_dims.first() {
        for kpi in chart_kpis.iter().take(2) {
            reports.push(ReportSuggestion {
                title: format!("{} over Time", kpi.display_name()),
                visualization: Visualization::LineChart,
                measure: Some(kpi.display_name().to_string()),
                dimension: Some(dim.clone()),
                time_granularity: Some(gran.clone()),
                confidence: (kpi.confidence * 0.96).min(0.96),
                reason: format!(
                    "measure `{}` plus time dimension `{}` → line chart at {} grain",
                    kpi.display_name(), dim, gran
                ),
            });
        }
    }

    let categories = categorical_dims(model, profiles);
    for kpi in chart_kpis.iter().take(2) {
        for (dim, distinct) in categories.iter().take(3) {
            let viz = if *distinct >= 3 && *distinct <= 8 && dim.to_ascii_lowercase().contains("region")
            {
                Visualization::PieChart
            } else {
                Visualization::BarChart
            };
            reports.push(ReportSuggestion {
                title: format!("{} by {}", kpi.display_name(), pretty(dim)),
                visualization: viz,
                measure: Some(kpi.display_name().to_string()),
                dimension: Some(dim.clone()),
                time_granularity: None,
                confidence: (kpi.confidence * 0.9).min(0.94),
                reason: if viz == Visualization::PieChart {
                    format!(
                        "measure `{}` and low-cardinality share dimension `{}` ({} values) → pie/donut",
                        kpi.display_name(), dim, distinct
                    )
                } else {
                    format!(
                        "measure `{}` and categorical dimension `{}` ({} values) → bar chart",
                        kpi.display_name(), dim, distinct
                    )
                },
            });
        }
    }

    if base_kpis.len() >= 2 || categories.len() >= 2 {
        reports.push(ReportSuggestion {
            title: "Measures by Dimension".to_string(),
            visualization: Visualization::Table,
            measure: base_kpis.first().map(|k| k.display_name().to_string()),
            dimension: categories.first().map(|(d, _)| d.clone()),
            time_granularity: None,
            confidence: 0.7,
            reason: "multiple measures or dimensions are easiest to scan as a table".to_string(),
        });
    }

    let mut seen = std::collections::HashSet::new();
    reports.retain(|r| seen.insert(r.title.clone()));
    reports.truncate(12);
    reports
}

fn related_time_dimensions(
    model: &SemanticModel,
    profiles: &DatabaseProfile,
    kpis: &[&KpiCandidate],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for col in model.time_dimensions() {
        let gran = profiles
            .column(&col.ref_.table, &col.ref_.column)
            .and_then(|c| c.inferred_time_granularity.clone())
            .unwrap_or_else(|| "month".to_string());
        out.push((col.ref_.column.clone(), gran));
    }
    if out.is_empty() {
        for kpi in kpis {
            if let Some(src) = &kpi.source {
                if let Some(table) = model.entity(&src.table) {
                    let _ = table;
                }
            }
        }
    }
    out.sort();
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

fn categorical_dims(model: &SemanticModel, profiles: &DatabaseProfile) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for col in &model.columns {
        if !matches!(
            col.role,
            SemanticRole::CategoricalDimension | SemanticRole::Dimension
        ) {
            continue;
        }
        let name = col.ref_.column.to_ascii_lowercase();
        if [
            "name", "email", "description", "comment", "notes", "active", "is_active",
            "discontinued", "resolved", "behoben", "gift_wrap", "sealed",
        ]
        .contains(&name.as_str())
            || name.starts_with("is_")
            || name.ends_with("_kz")
            || name.contains("aktiv")
            || name.contains("satz")
            || name.contains("mwst")
            || name.contains("prio")
            || name.contains("fakt")
            || name.contains("mahn")
            || name.contains("license")
            || name.contains("klasse")
        {
            continue;
        }
        let distinct = profiles
            .column(&col.ref_.table, &col.ref_.column)
            .map(|c| c.distinct_count)
            .unwrap_or(0);
        if distinct >= 2 && distinct <= 40 {
            out.push((col.ref_.column.clone(), distinct));
        }
    }
    out.sort_by(|(a, da), (b, db)| {
        share_rank(a)
            .cmp(&share_rank(b))
            .then_with(|| da.cmp(db))
    });
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

fn share_rank(name: &str) -> u8 {
    let n = name.to_ascii_lowercase();
    if n.contains("region") {
        0
    } else if n.contains("category") || n.contains("status") || n.contains("stage") {
        1
    } else {
        2
    }
}

fn pretty(name: &str) -> String {
    let mut base = name.to_string();
    let lower = base.to_ascii_lowercase();
    for suffix in ["_txt", "_kz", "_id"] {
        if lower.ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            break;
        }
    }
    let labeled = base.replace('_', " ");
    let mut chars = labeled.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => name.to_string(),
    }
}
