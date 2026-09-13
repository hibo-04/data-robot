use serde::{Deserialize, Serialize};

use crate::connector::{RawColumnProfile, RawDatabaseProfile, ValueCount};
use crate::schema::DatabaseSchema;
use crate::types::DataType;

mod classify;

pub use classify::{classify_column, is_key_name, looks_like_measure, ColumnRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseProfile {
    pub tables: Vec<TableProfile>,
}

impl DatabaseProfile {
    pub fn table(&self, name: &str) -> Option<&TableProfile> {
        self.tables.iter().find(|t| t.table == name)
    }

    pub fn column(&self, table: &str, column: &str) -> Option<&ColumnProfile> {
        self.table(table)?.columns.iter().find(|c| c.column == column)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableProfile {
    pub table: String,
    pub row_count: u64,
    pub columns: Vec<ColumnProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub table: String,
    pub column: String,
    pub data_type: DataType,
    pub row_count: u64,
    pub null_count: u64,
    pub null_ratio: f64,
    pub distinct_count: u64,
    pub distinct_ratio: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stddev: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_span_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_time_granularity: Option<String>,
    pub top_values: Vec<ValueCount>,
    pub role: ColumnRole,
    pub role_reason: String,
}

pub fn build_profiles(schema: &DatabaseSchema, raw: &RawDatabaseProfile) -> DatabaseProfile {
    let mut tables = Vec::new();
    for table in &schema.tables {
        let raw_table = match raw.table(&table.name) {
            Some(t) => t,
            None => continue,
        };
        let mut columns = Vec::new();
        for col in &table.columns {
            let raw_col = match raw_table.column(&col.name) {
                Some(c) => c,
                None => continue,
            };
            columns.push(enrich_column(schema, &table.name, &col.name, &col.data_type, col.nullable, raw_col));
        }
        tables.push(TableProfile {
            table: table.name.clone(),
            row_count: raw_table.row_count,
            columns,
        });
    }
    DatabaseProfile { tables }
}

fn enrich_column(
    schema: &DatabaseSchema,
    table: &str,
    column: &str,
    data_type: &DataType,
    nullable: bool,
    raw: &RawColumnProfile,
) -> ColumnProfile {
    let (time_span_days, granularity) = time_meta(raw);
    let (role, role_reason) = classify_column(schema, table, column, data_type, nullable, raw);
    ColumnProfile {
        table: table.to_string(),
        column: column.to_string(),
        data_type: data_type.clone(),
        row_count: raw.row_count,
        null_count: raw.null_count,
        null_ratio: round4(raw.null_ratio()),
        distinct_count: raw.distinct_count,
        distinct_ratio: round4(raw.distinct_ratio()),
        min: raw.min.clone(),
        max: raw.max.clone(),
        avg: raw.avg,
        stddev: raw.stddev,
        median: raw.median,
        avg_length: raw.avg_length,
        min_date: raw.min_date.clone(),
        max_date: raw.max_date.clone(),
        time_span_days,
        inferred_time_granularity: granularity,
        top_values: raw.top_values.clone(),
        role,
        role_reason,
    }
}

fn time_meta(raw: &RawColumnProfile) -> (Option<i64>, Option<String>) {
    let (Some(min), Some(max)) = (&raw.min_date, &raw.max_date) else {
        return (None, None);
    };
    let min = chrono::NaiveDate::parse_from_str(min, "%Y-%m-%d").ok();
    let max = chrono::NaiveDate::parse_from_str(max, "%Y-%m-%d").ok();
    match (min, max) {
        (Some(a), Some(b)) => {
            let days = (b - a).num_days();
            let gran = if days <= 2 {
                "day"
            } else if days <= 90 {
                "day"
            } else if days <= 800 {
                "month"
            } else {
                "year"
            };
            (Some(days), Some(gran.to_string()))
        }
        _ => (None, None),
    }
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}
