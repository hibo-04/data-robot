use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::kpi::KpiCandidate;
use crate::pipeline::Analysis;
use crate::relationships::Relationship;
use crate::types::ColumnRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    #[serde(default)]
    pub relationships: Vec<GroundTruthRelationship>,
    #[serde(default)]
    pub kpis: Vec<GroundTruthKpi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthRelationship {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthKpi {
    pub name: String,
    #[serde(default)]
    pub definition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub fixture: String,
    pub relationships: DetectionScores,
    pub kpis: DetectionScores,
    pub relationship_false_positives: Vec<String>,
    pub relationship_false_negatives: Vec<String>,
    pub kpi_false_positives: Vec<String>,
    pub kpi_false_negatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionScores {
    pub precision: f64,
    pub recall: f64,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

pub fn load_ground_truth(fixture_dir: impl AsRef<Path>) -> Result<Option<GroundTruth>> {
    let path = fixture_dir.as_ref().join("ground_truth.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

pub fn evaluate(analysis: &Analysis, truth: &GroundTruth) -> BenchmarkReport {
    let predicted_rels: Vec<String> = analysis
        .relationships
        .accepted()
        .map(rel_key)
        .collect();
    let truth_rels: Vec<String> = truth
        .relationships
        .iter()
        .map(|r| normalize_pair(&r.from, &r.to))
        .collect();
    let (rel_scores, rel_fp, rel_fn) = scores(&predicted_rels, &truth_rels);

    let predicted_kpis: Vec<&KpiCandidate> = analysis
        .kpis
        .iter()
        .filter(|k| k.confidence >= 0.75)
        .collect();
    let kpi_tp_pred = predicted_kpis
        .iter()
        .filter(|k| kpi_hits_truth_row(k, &truth.kpis))
        .count();
    let kpi_fp: Vec<String> = predicted_kpis
        .iter()
        .filter(|k| !kpi_hits_truth_row(k, &truth.kpis))
        .map(|k| k.display_name().to_string())
        .collect();
    let kpi_fn: Vec<String> = truth
        .kpis
        .iter()
        .filter(|t| !predicted_kpis.iter().any(|k| kpi_hits_truth_row(k, std::slice::from_ref(*t))))
        .map(|t| t.name.clone())
        .collect();
    let kpi_tp_truth = truth.kpis.len() - kpi_fn.len();
    let kpi_scores = DetectionScores {
        precision: if predicted_kpis.is_empty() {
            0.0
        } else {
            round4(kpi_tp_pred as f64 / predicted_kpis.len() as f64)
        },
        recall: if truth.kpis.is_empty() {
            1.0
        } else {
            round4(kpi_tp_truth as f64 / truth.kpis.len() as f64)
        },
        true_positives: kpi_tp_truth,
        false_positives: kpi_fp.len(),
        false_negatives: kpi_fn.len(),
    };

    BenchmarkReport {
        fixture: analysis.source_name.clone(),
        relationships: rel_scores,
        kpis: kpi_scores,
        relationship_false_positives: rel_fp,
        relationship_false_negatives: rel_fn,
        kpi_false_positives: kpi_fp,
        kpi_false_negatives: kpi_fn,
    }
}

fn rel_key(rel: &Relationship) -> String {
    normalize_pair(&rel.from.qualified(), &rel.to.qualified())
}

fn kpi_hits_truth_row(kpi: &KpiCandidate, truth: &[GroundTruthKpi]) -> bool {
    truth.iter().any(|t| kpi_matches_truth(kpi, t))
}

fn kpi_matches_truth(kpi: &KpiCandidate, truth: &GroundTruthKpi) -> bool {
    let names = kpi_aliases(kpi);
    let want = normalize_name(&truth.name);
    if names.iter().any(|a| a == &want) {
        return true;
    }
    if let Some(def) = &truth.definition {
        let def_n = normalize_expr(def);
        if let Some(expr) = &kpi.expression {
            if normalize_expr(expr) == def_n {
                return true;
            }
        }
        if let Some(src) = source_from_definition(def) {
            if kpi
                .source
                .as_ref()
                .is_some_and(|s| s.qualified().eq_ignore_ascii_case(&src))
            {
                return true;
            }
        }
    }
    false
}

fn source_from_definition(def: &str) -> Option<String> {
    let start = def.find('(')? + 1;
    let end = def.rfind(')')?;
    let inner = def[start..end].trim();
    let inner = inner
        .trim_start_matches("DISTINCT")
        .trim_start_matches("distinct")
        .trim();
    if inner.contains('.') {
        Some(inner.to_string())
    } else {
        None
    }
}

fn normalize_expr(expr: &str) -> String {
    expr.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.')
        .collect()
}

fn kpi_aliases(kpi: &KpiCandidate) -> Vec<String> {
    let mut keys = vec![normalize_name(&kpi.name), normalize_name(kpi.display_name())];
    if let Some(label) = &kpi.business_label {
        keys.push(normalize_name(label));
    }
    keys.sort();
    keys.dedup();
    keys
}

fn normalize_pair(from: &str, to: &str) -> String {
    format!("{}->{}", from.to_ascii_lowercase(), to.to_ascii_lowercase())
}

fn normalize_name(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', ' '], "")
}

fn scores(predicted: &[String], truth: &[String]) -> (DetectionScores, Vec<String>, Vec<String>) {
    let mut tp = 0usize;
    let mut fp = Vec::new();
    for p in predicted {
        if truth.iter().any(|t| t == p) {
            tp += 1;
        } else {
            fp.push(p.clone());
        }
    }
    let mut fn_ = Vec::new();
    for t in truth {
        if !predicted.iter().any(|p| p == t) {
            fn_.push(t.clone());
        }
    }
    let precision = if predicted.is_empty() {
        0.0
    } else {
        tp as f64 / predicted.len() as f64
    };
    let recall = if truth.is_empty() {
        1.0
    } else {
        tp as f64 / truth.len() as f64
    };
    (
        DetectionScores {
            precision: round4(precision),
            recall: round4(recall),
            true_positives: tp,
            false_positives: fp.len(),
            false_negatives: fn_.len(),
        },
        fp,
        fn_,
    )
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[allow(dead_code)]
fn column_ref(qualified: &str) -> Option<ColumnRef> {
    qualified.split_once('.').map(|(t, c)| ColumnRef::new(t, c))
}
