use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::identities::{ColumnIdentity, IdentityScope, IdentityTemplate};
use crate::semantic::{entity_name, SemanticModel, SemanticRole};
use crate::types::ColumnRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aggregation {
    Sum,
    Avg,
    Count,
    CountDistinct,
    Min,
    Max,
}

impl Aggregation {
    pub fn sql_fn(&self) -> &'static str {
        match self {
            Self::Sum => "SUM",
            Self::Avg => "AVG",
            Self::Count => "COUNT",
            Self::CountDistinct => "COUNT",
            Self::Min => "MIN",
            Self::Max => "MAX",
        }
    }

    pub fn as_id_prefix(&self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Count => "count",
            Self::CountDistinct => "count_distinct",
            Self::Min => "min",
            Self::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    Sicher,
    Wahrscheinlich,
    Unsicher,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiCandidate {
    pub id: String,
    /// Domain-agnostic name from entity + column (or derived rule name).
    pub name: String,
    /// Optional overlay from a dictionary pack. Never required for the core.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dictionary_pack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dictionary_entry: Option<String>,
    pub source: Option<ColumnRef>,
    pub aggregation: Option<Aggregation>,
    pub expression: Option<String>,
    pub confidence: f64,
    pub certainty: Certainty,
    pub reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub inputs: Vec<String>,
}

impl KpiCandidate {
    pub fn display_name(&self) -> &str {
        self.business_label.as_deref().unwrap_or(&self.name)
    }

    pub fn matches_name(&self, needle: &str) -> bool {
        let n = needle.trim();
        self.name.eq_ignore_ascii_case(n)
            || self.display_name().eq_ignore_ascii_case(n)
            || self
                .business_label
                .as_deref()
                .is_some_and(|l| l.eq_ignore_ascii_case(n))
            || self.id.eq_ignore_ascii_case(n)
            || self
                .source
                .as_ref()
                .is_some_and(|s| s.column.eq_ignore_ascii_case(n) || s.qualified().eq_ignore_ascii_case(n))
    }
}

/// Structural KPIs only: aggregation + grain from the model. No industry names.
pub fn detect_kpis(model: &SemanticModel) -> Vec<KpiCandidate> {
    let mut kpis = Vec::new();

    for col in &model.columns {
        if col.role == SemanticRole::Measure
            && !skip_sum_measure(&col.ref_.table, &col.ref_.column, model)
        {
            kpis.push(measure_kpi(&col.ref_.table, &col.ref_.column, &col.reason));
        }
        if col.role == SemanticRole::Identifier
            && looks_like_countable_event(&col.ref_.table, model)
        {
            let entity = entity_name(&col.ref_.table);
            kpis.push(KpiCandidate {
                id: format!("count_distinct:{}", col.ref_.qualified()),
                name: format!("{entity} Count"),
                business_label: None,
                dictionary_pack: None,
                dictionary_entry: None,
                source: Some(col.ref_.clone()),
                aggregation: Some(Aggregation::CountDistinct),
                expression: Some(format!("COUNT(DISTINCT {})", col.ref_.qualified())),
                confidence: 0.8,
                certainty: Certainty::Wahrscheinlich,
                reason: format!(
                    "primary identifier `{}` on a primary event table is counted distinctly as `{entity} Count`",
                    col.ref_.qualified()
                ),
                inputs: Vec::new(),
            });
        }
        if col.role == SemanticRole::ForeignKey && looks_like_fact_table(&col.ref_.table, model) {
            if let Some(entity) = infer_target_entity(model, &col.ref_) {
                kpis.push(KpiCandidate {
                    id: format!("count_distinct:{}", col.ref_.qualified()),
                    name: format!("{entity} Count"),
                    business_label: None,
                    dictionary_pack: None,
                    dictionary_entry: None,
                    source: Some(col.ref_.clone()),
                    aggregation: Some(Aggregation::CountDistinct),
                    expression: Some(format!("COUNT(DISTINCT {})", col.ref_.qualified())),
                    confidence: 0.72,
                    certainty: Certainty::Wahrscheinlich,
                    reason: format!(
                        "distinct `{}` counts unique `{}` referenced from event table `{}`",
                        col.ref_.qualified(),
                        entity,
                        col.ref_.table
                    ),
                    inputs: Vec::new(),
                });
            }
        }
    }

    dedup_kpis(kpis)
}

/// Drop SUM KPIs that reconstruct another kept measure (grain copies, additive parts, products).
pub fn drop_double_counted_kpis(kpis: &mut Vec<KpiCandidate>, identities: &[ColumnIdentity]) {
    let sum_sources: HashSet<String> = kpis
        .iter()
        .filter(|k| k.aggregation == Some(Aggregation::Sum))
        .filter_map(|k| k.source.as_ref().map(|s| s.qualified()))
        .collect();

    let mut drop: HashSet<String> = HashSet::new();
    for id in identities {
        if id.template == IdentityTemplate::GrainSum {
            let parent = id.target.qualified();
            let Some(child) = id.inputs.first().map(|c| c.qualified()) else {
                continue;
            };
            if sum_sources.contains(&parent) {
                drop.insert(child.clone());
                expand_intra_additive_family(&mut drop, identities, &child);
            } else if sum_sources.contains(&child) {
                drop.insert(parent);
            }
        }
        if matches!(id.template, IdentityTemplate::Sum | IdentityTemplate::Sum3)
            && id.scope == IdentityScope::IntraTable
        {
            let target = id.target.qualified();
            if sum_sources.contains(&target) {
                for inp in &id.inputs {
                    drop.insert(inp.qualified());
                }
            }
        }
        if matches!(id.template, IdentityTemplate::Product | IdentityTemplate::ScaledProduct) {
            let same_table_input = id.inputs.iter().any(|inp| {
                inp.table == id.target.table && sum_sources.contains(&inp.qualified())
            });
            if same_table_input {
                drop.insert(id.target.qualified());
            }
        }
    }

    kpis.retain(|k| {
        k.source
            .as_ref()
            .map(|s| !drop.contains(&s.qualified()))
            .unwrap_or(true)
    });
}

pub fn dedup_kpis(mut kpis: Vec<KpiCandidate>) -> Vec<KpiCandidate> {
    kpis.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.display_name().cmp(b.display_name()))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut seen_names = HashSet::new();
    let mut seen_labels = HashSet::new();
    kpis.retain(|k| {
        seen_names.insert(k.name.clone()) && seen_labels.insert(k.display_name().to_ascii_lowercase())
    });
    kpis
}

fn expand_intra_additive_family(
    drop: &mut HashSet<String>,
    identities: &[ColumnIdentity],
    seed: &str,
) {
    let seed_table = seed.split('.').next().unwrap_or(seed);
    let mut changed = true;
    while changed {
        changed = false;
        for id in identities {
            if id.scope != IdentityScope::IntraTable {
                continue;
            }
            if !matches!(
                id.template,
                IdentityTemplate::Sum
                    | IdentityTemplate::Sum3
                    | IdentityTemplate::Rate
                    | IdentityTemplate::Difference
            ) {
                continue;
            }
            if id.target.table != seed_table {
                continue;
            }
            let cols: Vec<String> = std::iter::once(id.target.qualified())
                .chain(id.inputs.iter().map(|c| c.qualified()))
                .collect();
            if cols.iter().any(|c| c == seed || drop.contains(c)) {
                for c in cols {
                    if drop.insert(c) {
                        changed = true;
                    }
                }
            }
        }
    }
}

fn measure_kpi(table: &str, column: &str, role_reason: &str) -> KpiCandidate {
    let source = ColumnRef::new(table, column);
    let entity = entity_name(table);
    let pretty = pretty_column(column);
    let name = format!("{entity} {pretty}");
    KpiCandidate {
        id: format!("sum:{}", source.qualified()),
        name,
        business_label: None,
        dictionary_pack: None,
        dictionary_entry: None,
        source: Some(source.clone()),
        aggregation: Some(Aggregation::Sum),
        expression: Some(format!("SUM({})", source.qualified())),
        confidence: 0.88,
        certainty: Certainty::Sicher,
        reason: format!(
            "structural measure `{}.{}` aggregated with SUM as `{}`. {role_reason}",
            table, column, pretty
        ),
        inputs: Vec::new(),
    }
}

const EVENT_TABLE_HINTS: &[&str] = &[
    "order", "item", "invoice", "payment", "ship", "stop", "scan", "ticket", "tix", "entry",
    "reading", "fault", "stoer", "piece", "return", "subscri", "abo", "usage", "nutz", "activit",
    "opportunit", "line", "route", "tour", "time", "zeit",
];

/// Primary business-event tables whose PK is counted. Child grains (items, stops, readings) are not.
const COUNTABLE_EVENT_TABLES: &[&str] = &[
    "order",
    "orders",
    "shipment",
    "shipments",
    "ticket",
    "tickets",
    "opportunity",
    "opportunities",
];

const DURATION_EVENT_HINTS: &[&str] = &["fault", "stoer", "down"];
const RETURN_TABLE_HINTS: &[&str] = &["return", "refund", "retoure"];
const PAYMENT_TABLE_HINTS: &[&str] = &["payment", "zahlung"];
const SNAPSHOT_TABLE_HINTS: &[&str] = &["budget"];

fn looks_like_fact_table(table: &str, model: &SemanticModel) -> bool {
    if !table_has_outgoing_fk(table, model) {
        return false;
    }
    let has_measure = model
        .columns
        .iter()
        .any(|c| c.ref_.table == table && c.role == SemanticRole::Measure);
    if !has_measure {
        return false;
    }
    let n = table.to_ascii_lowercase();
    EVENT_TABLE_HINTS.iter().any(|h| n.contains(h))
}

fn looks_like_countable_event(table: &str, model: &SemanticModel) -> bool {
    if !looks_like_fact_table(table, model) {
        return false;
    }
    let n = table.to_ascii_lowercase();
    COUNTABLE_EVENT_TABLES.iter().any(|t| n == *t)
}

fn table_has_outgoing_fk(table: &str, model: &SemanticModel) -> bool {
    model
        .relationships
        .iter()
        .any(|r| r.from_column.table == table)
}

fn table_hint(table: &str, hints: &[&str]) -> bool {
    let n = table.to_ascii_lowercase();
    hints.iter().any(|h| n.contains(h))
}

fn skip_sum_measure(table: &str, column: &str, model: &SemanticModel) -> bool {
    if !table_has_outgoing_fk(table, model) {
        return true;
    }
    let n = column.to_ascii_lowercase();
    if token_in(&n, DURATION_TOKENS) && !table_hint(table, DURATION_EVENT_HINTS) {
        return true;
    }
    if token_in(&n, QTY_TOKENS) && table_hint(table, RETURN_TABLE_HINTS) {
        return true;
    }
    if table_hint(table, PAYMENT_TABLE_HINTS) || table_hint(table, SNAPSHOT_TABLE_HINTS) {
        return true;
    }
    if n.contains("weight") || n.contains("gewicht") || n.contains("gew") {
        return true;
    }
    let skip = [
        "price", "preis", "einzelpr", "remaining", "rest", "unit_cost", "ek_preis", "list_price",
        "quota", "kredit", "stock", "lager", "cycle", "zyklus", "seats", "anz_seats", "inkl_seats",
        "paid_amount", "gezahlt", "scrap", "ausschuss", "tax", "mwst", "vat", "weight", "gewicht",
        "gew", "discount", "rabatt", "planned", "plan_stk", "plan", "spent", "verbraucht", "rating",
        "bewertung", "seq", "lfd", "dwell", "standzeit", "capacity", "events", "estimate", "logged",
        "freight", "fracht", "fee", "geb", "handle", "bearb", "subtotal", "nutzlast", "payload",
        "lieferzeit", "nenn", "budget", "std_ist", "kap",
    ];
    skip.iter()
        .any(|t| n == *t || n.split(['_', '-']).any(|p| p == *t) || (t.len() >= 5 && n.contains(t)))
}

const DURATION_TOKENS: &[&str] = &["duration", "dauer", "dauer_min"];
const QTY_TOKENS: &[&str] = &["quantity", "qty", "menge", "units"];

fn token_in(name: &str, tokens: &[&str]) -> bool {
    let parts: Vec<&str> = name.split(['_', '-', ' ']).filter(|p| !p.is_empty()).collect();
    tokens.iter().any(|t| {
        name == *t || parts.iter().any(|p| p == t) || (t.len() >= 4 && name.contains(t))
    })
}

fn infer_target_entity(model: &SemanticModel, from: &ColumnRef) -> Option<String> {
    model
        .relationships
        .iter()
        .find(|r| r.from_column.table == from.table && r.from_column.column == from.column)
        .map(|r| r.to_entity.clone())
}

fn pretty_column(column: &str) -> String {
    column
        .split(['_', '-', ' '])
        .filter(|p| !p.is_empty())
        .map(title_case)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

impl Certainty {
    pub fn from_confidence(c: f64) -> Self {
        if c >= 0.85 {
            Self::Sicher
        } else if c >= 0.65 {
            Self::Wahrscheinlich
        } else {
            Self::Unsicher
        }
    }
}
