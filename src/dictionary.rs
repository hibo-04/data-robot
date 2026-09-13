//! Optional vocabulary overlay on top of domain-agnostic structural KPIs.
//!
//! Packs never invent measures. They only attach business labels (and derived KPIs)
//! when tokens match **and** entity hints match. Core analysis works with no packs.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AnalyticsError, Result};
use crate::kpi::{Aggregation, Certainty, KpiCandidate};
use crate::semantic::{entity_name, SemanticModel, SemanticRole};

const GENERIC_JSON: &str = include_str!("../dictionaries/generic.json");
const COMMERCE_JSON: &str = include_str!("../dictionaries/commerce.json");
const OPERATIONS_JSON: &str = include_str!("../dictionaries/operations.json");
const LOGISTICS_JSON: &str = include_str!("../dictionaries/logistics.json");
const SAAS_JSON: &str = include_str!("../dictionaries/saas.json");
const SERVICES_JSON: &str = include_str!("../dictionaries/services.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryPack {
    pub pack: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entries: Vec<DictionaryEntry>,
    #[serde(default)]
    pub derived: Vec<DerivedKpiRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: String,
    pub tokens: Vec<String>,
    #[serde(default)]
    pub required_role: Option<String>,
    #[serde(default)]
    pub entity_hints: Vec<String>,
    #[serde(default)]
    pub aggregation: Option<Aggregation>,
    pub label: String,
    #[serde(default)]
    pub confidence_boost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedKpiRule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub requires_labels: Vec<String>,
    #[serde(default)]
    pub requires_structural_names: Vec<String>,
    pub expression: String,
    #[serde(default = "default_derived_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
}

fn default_derived_confidence() -> f64 {
    0.8
}

#[derive(Debug, Clone, Default)]
pub struct DictionarySet {
    packs: Vec<DictionaryPack>,
}

impl DictionarySet {
    pub fn bundled() -> Result<Self> {
        Self::from_json_docs(&[
            ("generic", GENERIC_JSON),
            ("commerce", COMMERCE_JSON),
            ("operations", OPERATIONS_JSON),
            ("logistics", LOGISTICS_JSON),
            ("saas", SAAS_JSON),
            ("services", SERVICES_JSON),
        ])
    }

    pub fn from_json_docs(docs: &[(&str, &str)]) -> Result<Self> {
        let mut packs = Vec::new();
        for (_, json) in docs {
            packs.push(serde_json::from_str(json)?);
        }
        Ok(Self { packs })
    }

    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let mut packs = Vec::new();
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(AnalyticsError::msg(format!(
                "dictionary directory `{}` does not exist",
                dir.display()
            )));
        }
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        files.sort();
        for path in files {
            let text = std::fs::read_to_string(&path)?;
            packs.push(serde_json::from_str(&text)?);
        }
        Ok(Self { packs })
    }

    pub fn with_packs(&self, names: &[&str]) -> Self {
        let wanted: Vec<String> = names.iter().map(|s| s.to_ascii_lowercase()).collect();
        Self {
            packs: self
                .packs
                .iter()
                .filter(|p| wanted.iter().any(|w| w == &p.pack.to_ascii_lowercase()))
                .cloned()
                .collect(),
        }
    }

    pub fn pack_names(&self) -> Vec<String> {
        self.packs.iter().map(|p| p.pack.clone()).collect()
    }

    /// Apply labels from selected packs. Later packs override earlier ones when
    /// the match is at least as specific (commerce after generic is the default).
    pub fn apply(&self, model: &SemanticModel, kpis: &mut [KpiCandidate]) {
        for kpi in kpis.iter_mut() {
            let mut best: Option<(i32, DictionaryEntry, String)> = None;
            for pack in &self.packs {
                if let Some((score, entry)) = best_entry(pack, model, kpi) {
                    let take = match &best {
                        None => true,
                        Some((prev, _, _)) => score >= *prev,
                    };
                    if take {
                        best = Some((score, entry, pack.pack.clone()));
                    }
                }
            }
            if let Some((_, entry, pack_name)) = best {
                kpi.business_label = Some(entry.label.clone());
                kpi.dictionary_pack = Some(pack_name.clone());
                kpi.dictionary_entry = Some(entry.id.clone());
                kpi.confidence = (kpi.confidence + entry.confidence_boost).min(0.98);
                kpi.certainty = Certainty::from_confidence(kpi.confidence);
                kpi.reason = format!(
                    "{}. dictionary `{}` / `{}` → `{}`",
                    kpi.reason, pack_name, entry.id, entry.label
                );
            }
        }
    }

    pub fn derived_kpis(&self, existing: &[KpiCandidate]) -> Vec<KpiCandidate> {
        let mut out = Vec::new();
        for pack in &self.packs {
            for rule in &pack.derived {
                if let Some(kpi) = try_derived(existing, pack, rule) {
                    if !out.iter().any(|k: &KpiCandidate| k.id == kpi.id)
                        && !existing.iter().any(|k| k.id == kpi.id)
                    {
                        out.push(kpi);
                    }
                }
            }
        }
        out
    }
}

fn best_entry(
    pack: &DictionaryPack,
    model: &SemanticModel,
    kpi: &KpiCandidate,
) -> Option<(i32, DictionaryEntry)> {
    let mut best: Option<(i32, DictionaryEntry)> = None;
    for entry in &pack.entries {
        if let Some(score) = entry_score(entry, model, kpi) {
            if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                best = Some((score, entry.clone()));
            }
        }
    }
    best
}

fn entry_score(entry: &DictionaryEntry, model: &SemanticModel, kpi: &KpiCandidate) -> Option<i32> {
    if let (Some(req), Some(agg)) = (&entry.aggregation, kpi.aggregation) {
        if *req != agg {
            return None;
        }
    }
    if let Some(role) = &entry.required_role {
        if !role_matches(role, kpi, model) {
            return None;
        }
    }
    let Some(source) = &kpi.source else {
        return None;
    };
    let column = source.column.to_ascii_lowercase();
    let entity = entity_name(&source.table);
    let hint_ok = entity_hints_match(&entry.entity_hints, &entity, &source.table);
    if !entry.entity_hints.is_empty() && !hint_ok {
        return None;
    }
    let token_score = token_match_score(&column, &entity, &entry.tokens)?;
    let mut score = token_score;
    if hint_ok && !entry.entity_hints.is_empty() {
        score += 100;
    }
    Some(score)
}

fn role_matches(required: &str, kpi: &KpiCandidate, model: &SemanticModel) -> bool {
    let req = required.to_ascii_lowercase();
    let Some(source) = &kpi.source else {
        return false;
    };
    let Some(col) = model.column(&source.table, &source.column) else {
        return matches!(req.as_str(), "measure") && kpi.aggregation == Some(Aggregation::Sum);
    };
    match req.as_str() {
        "measure" => col.role == SemanticRole::Measure,
        "identifier" => col.role == SemanticRole::Identifier,
        "foreign_key" => col.role == SemanticRole::ForeignKey,
        _ => true,
    }
}

fn entity_hints_match(hints: &[String], entity: &str, table: &str) -> bool {
    if hints.is_empty() {
        return true;
    }
    let entity_n = norm(entity);
    let table_n = norm(table);
    hints.iter().any(|h| {
        let h = norm(h);
        entity_n.contains(&h) || table_n.contains(&h) || h.contains(&entity_n)
    })
}

fn token_match_score(column: &str, entity: &str, tokens: &[String]) -> Option<i32> {
    let col = column.to_ascii_lowercase();
    let parts: Vec<String> = col
        .split(['_', '-', ' ', '.'])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let entity_n = entity.to_ascii_lowercase();
    let mut best = 0i32;
    for token in tokens {
        let t = token.to_ascii_lowercase();
        if col == t {
            best = best.max(30 + t.len() as i32);
        } else if parts.iter().any(|p| p == &t) {
            best = best.max(20 + t.len() as i32);
        } else if col.contains(&t) && t.len() >= 3 {
            best = best.max(10 + t.len() as i32);
        } else if entity_n.contains(&t) && t.len() >= 4 {
            best = best.max(8 + t.len() as i32);
        }
    }
    if best == 0 {
        None
    } else {
        Some(best)
    }
}

fn try_derived(
    existing: &[KpiCandidate],
    pack: &DictionaryPack,
    rule: &DerivedKpiRule,
) -> Option<KpiCandidate> {
    for label in &rule.requires_labels {
        if !existing.iter().any(|k| k.matches_name(label)) {
            return None;
        }
    }
    for name in &rule.requires_structural_names {
        if !existing.iter().any(|k| k.name.eq_ignore_ascii_case(name)) {
            return None;
        }
    }
    let inputs: Vec<String> = rule
        .requires_labels
        .iter()
        .cloned()
        .chain(rule.requires_structural_names.iter().cloned())
        .collect();
    let confidence = rule.confidence.min(
        existing
            .iter()
            .filter(|k| inputs.iter().any(|i| k.matches_name(i)))
            .map(|k| k.confidence)
            .fold(1.0, f64::min),
    );
    Some(KpiCandidate {
        id: rule.id.clone(),
        name: rule.name.clone(),
        business_label: Some(rule.name.clone()),
        dictionary_pack: Some(pack.pack.clone()),
        dictionary_entry: Some(rule.id.clone()),
        source: None,
        aggregation: None,
        expression: Some(rule.expression.clone()),
        confidence,
        certainty: Certainty::from_confidence(confidence),
        reason: if rule.reason.is_empty() {
            format!("derived KPI from dictionary pack `{}`", pack.pack)
        } else {
            rule.reason.clone()
        },
        inputs,
    })
}

fn norm(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

pub fn parse_pack_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

pub const DEFAULT_PACKS: &str = "generic,commerce,operations,logistics,saas,services";
