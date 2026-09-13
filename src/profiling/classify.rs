use crate::connector::RawColumnProfile;
use crate::schema::DatabaseSchema;
use crate::types::DataType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRole {
    Identifier,
    ForeignKeyCandidate,
    MeasureCandidate,
    DimensionCandidate,
    TimeDimensionCandidate,
    CategoricalDimension,
    Unknown,
}

/// Additive measure tokens. Short tokens match whole parts only (`qty`, `km`, `mrr`).
const MEASURE_NAMES: &[&str] = &[
    "amount",
    "revenue",
    "netto",
    "net_amount",
    "gross",
    "gross_amount",
    "total",
    "price",
    "cost",
    "costs",
    "kosten",
    "qty",
    "quantity",
    "units",
    "menge",
    "discount",
    "tax",
    "margin",
    "profit",
    "refund",
    "shipping",
    "fee",
    "quota",
    "salary",
    "balance",
    "hours",
    "stunden",
    "delay",
    "versp",
    "duration",
    "dauer",
    "energy",
    "energie",
    "mrr",
    "arr",
    "seats",
    "km",
    "distance",
    "distanz",
    "weight",
    "gewicht",
    "events",
    "betrag",
    "erstattung",
];

const FLAG_NAMES: &[&str] = &[
    "flag", "aktiv", "active", "resolved", "behoben", "discontinued", "ok", "io", "kz", "prio",
    "dunning", "mahnstufe", "gift", "sealed",
];

const RATE_NAMES: &[&str] = &[
    "rate", "satz", "pct", "percent", "probability", "ausbeute", "yield", "allocation", "vat",
];

const NON_ADDITIVE_NAMES: &[&str] = &[
    "temp",
    "temperature",
    "vibration",
    "vib",
    "rms",
    "cycle",
    "power",
    "leist",
    "nennleist",
    "payload",
    "nutzlast",
];

const KEY_SUFFIXES: &[&str] = &[
    "_id", "_ids", "_nr", "_no", "_num", "_number", "_key", "_fk", "_pk", "_code", "_sku", "_uuid",
];

pub fn classify_column(
    schema: &DatabaseSchema,
    table: &str,
    column: &str,
    data_type: &DataType,
    _nullable: bool,
    raw: &RawColumnProfile,
) -> (ColumnRole, String) {
    let lower = column.to_ascii_lowercase();
    if schema.is_primary_key(table, column) {
        return (
            ColumnRole::Identifier,
            format!("column `{table}.{column}` is part of the primary key"),
        );
    }
    if schema
        .table(table)
        .map(|t| t.foreign_keys.iter().any(|fk| fk.columns.iter().any(|c| c == column)))
        .unwrap_or(false)
    {
        return (
            ColumnRole::ForeignKeyCandidate,
            format!("column `{table}.{column}` is a declared foreign key"),
        );
    }
    if data_type.is_temporal() || looks_like_time_name(&lower) {
        return (
            ColumnRole::TimeDimensionCandidate,
            format!("column `{table}.{column}` has a temporal type or date-like name"),
        );
    }

    let key_like = is_key_name(&lower);
    if key_like && (data_type.is_numeric() || data_type.is_textish()) {
        if raw.distinct_ratio() >= 0.95 && schema.is_unique_column(table, column) {
            return (
                ColumnRole::Identifier,
                format!("`{column}` looks like a unique identifier (name + uniqueness)"),
            );
        }
        return (
            ColumnRole::ForeignKeyCandidate,
            format!("`{column}` matches identifier naming (`*_id` / `*_nr` / `*_sku`) and is not the primary key"),
        );
    }

    if data_type == &DataType::Boolean || (data_type.is_numeric() && looks_like_flag(&lower, raw)) {
        return (
            ColumnRole::CategoricalDimension,
            format!("column `{column}` looks like a flag or small enum, not an additive measure"),
        );
    }

    if data_type.is_numeric() && looks_like_rate(&lower, raw) {
        return (
            ColumnRole::CategoricalDimension,
            format!("numeric column `{column}` looks like a rate/percent factor, not a SUM measure"),
        );
    }

    if data_type.is_numeric() && token_hit(&lower, NON_ADDITIVE_NAMES) {
        return (
            ColumnRole::Unknown,
            format!("numeric column `{column}` is observational (not additive), kept out of SUM KPIs"),
        );
    }

    if data_type.is_numeric() && looks_like_measure(&lower) {
        return (
            ColumnRole::MeasureCandidate,
            format!("numeric column `{column}` matches a measure name pattern"),
        );
    }

    let low_card_text = raw.distinct_count <= 50 || raw.distinct_ratio() <= 0.05;
    if (data_type.is_textish() || data_type == &DataType::Boolean) && low_card_text && !key_like {
        return (
            ColumnRole::CategoricalDimension,
            format!(
                "low cardinality ({} distinct, ratio {:.3}) suggests a categorical dimension",
                raw.distinct_count,
                raw.distinct_ratio()
            ),
        );
    }

    if data_type.is_textish() {
        return (
            ColumnRole::DimensionCandidate,
            format!("text column `{column}` is treated as a descriptive dimension"),
        );
    }

    if data_type.is_numeric() && !key_like && raw.stddev.unwrap_or(0.0).abs() > 1e-9 {
        return (
            ColumnRole::MeasureCandidate,
            format!(
                "numeric column `{column}` varies (stddev {:.3}) and is treated as additive",
                raw.stddev.unwrap_or(0.0)
            ),
        );
    }

    (
        ColumnRole::Unknown,
        format!("no strong rule matched for `{table}.{column}`"),
    )
}

pub fn is_key_name(name: &str) -> bool {
    if matches!(name, "id" | "pk" | "uuid" | "guid" | "sku" | "code") {
        return true;
    }
    KEY_SUFFIXES.iter().any(|s| name.ends_with(s))
}

pub fn looks_like_measure(name: &str) -> bool {
    token_hit(name, MEASURE_NAMES)
}

fn looks_like_flag(name: &str, raw: &RawColumnProfile) -> bool {
    if token_hit(name, FLAG_NAMES) && raw.distinct_count <= 8 {
        return true;
    }
    if raw.distinct_count <= 3 {
        if let (Some(min), Some(max)) = (parse_f64(raw.min.as_deref()), parse_f64(raw.max.as_deref())) {
            if min >= 0.0 && max <= 2.0 && (max - min) <= 2.0 {
                return true;
            }
        }
    }
    false
}

fn looks_like_rate(name: &str, raw: &RawColumnProfile) -> bool {
    if !token_hit(name, RATE_NAMES) && !name.ends_with("_pct") && !name.contains("pct") {
        return false;
    }
    let min = parse_f64(raw.min.as_deref());
    let max = parse_f64(raw.max.as_deref());
    match (min, max) {
        (Some(a), Some(b)) if a >= -0.001 && b <= 1.5 => true,
        (Some(a), Some(b)) if a >= 0.0 && b <= 100.0 => true,
        _ => true,
    }
}

fn token_hit(name: &str, tokens: &[&str]) -> bool {
    let parts: Vec<&str> = name.split(['_', '-', ' ', '.']).filter(|p| !p.is_empty()).collect();
    tokens.iter().any(|tok| {
        if name == *tok {
            return true;
        }
        if parts.iter().any(|p| p == tok) {
            return true;
        }
        tok.len() >= 4 && name.contains(tok)
    })
}

fn parse_f64(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|s| s.parse().ok())
}

fn looks_like_time_name(name: &str) -> bool {
    ["date", "time", "timestamp", "created", "updated", "closed", "hired", "eintritt"]
        .iter()
        .any(|p| name.contains(p))
}
