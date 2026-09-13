use crate::error::Result;
use crate::limits::MAX_DISTINCT_FOR_OVERLAP;
use crate::query::{QueryPlan, QueryResult};
use crate::schema::DatabaseSchema;
use crate::types::SqlDialect;

mod fixture;
mod postgres;

pub use fixture::FixtureConnector;
pub use postgres::{map_postgres_type, PostgresConnector};

/// Source-agnostic access to schema metadata, profiles, value sets, and queries.
///
/// Analytics engines must depend on this trait only — never on PostgreSQL or
/// fixture file formats.
pub trait DataSource: Send + Sync {
    fn name(&self) -> &str;
    fn dialect(&self) -> SqlDialect;

    fn schema(&self) -> Result<DatabaseSchema>;

    /// Profile every table in as few scans as the source can manage.
    fn profile_raw(&self) -> Result<RawDatabaseProfile>;

    /// Distinct values (plus frequencies) used by relationship overlap scoring.
    fn distinct_values(
        &self,
        table: &str,
        column: &str,
        max_values: usize,
    ) -> Result<DistinctSet>;

    fn execute_sql(&self, sql: &str) -> Result<QueryResult>;

    /// Execute a planned query. Sources may run SQL or an in-process plan.
    fn execute_plan(&self, plan: &QueryPlan) -> Result<QueryResult> {
        self.execute_sql(&plan.to_sql(self.dialect())?)
    }

    /// Numeric values used by formula detection. Keys and text are out of scope.
    fn sample_numeric(&self, table: &str, columns: &[String], max_rows: usize) -> Result<NumericSample>;

    /// Child ⋈ parent on a key, returning numeric columns from both sides.
    fn sample_join_numeric(
        &self,
        left_table: &str,
        left_columns: &[String],
        right_table: &str,
        right_columns: &[String],
        left_key: &str,
        right_key: &str,
        max_rows: usize,
    ) -> Result<NumericSample>;

    /// Parent measures aligned with `SUM(child.measure)` grouped by the relationship key.
    fn sample_grouped_sum(
        &self,
        child_table: &str,
        child_key: &str,
        child_measure: &str,
        parent_table: &str,
        parent_key: &str,
        parent_measures: &[String],
        max_groups: usize,
    ) -> Result<NumericSample>;
}

#[derive(Debug, Clone, Default)]
pub struct RawDatabaseProfile {
    pub tables: Vec<RawTableProfile>,
}

impl RawDatabaseProfile {
    pub fn table(&self, name: &str) -> Option<&RawTableProfile> {
        self.tables.iter().find(|t| t.table == name)
    }
}

#[derive(Debug, Clone)]
pub struct RawTableProfile {
    pub table: String,
    pub row_count: u64,
    pub columns: Vec<RawColumnProfile>,
}

impl RawTableProfile {
    pub fn column(&self, name: &str) -> Option<&RawColumnProfile> {
        self.columns.iter().find(|c| c.column == name)
    }
}

#[derive(Debug, Clone)]
pub struct RawColumnProfile {
    pub column: String,
    pub row_count: u64,
    pub null_count: u64,
    pub distinct_count: u64,
    pub min: Option<String>,
    pub max: Option<String>,
    pub avg: Option<f64>,
    pub stddev: Option<f64>,
    pub median: Option<f64>,
    pub avg_length: Option<f64>,
    pub min_date: Option<String>,
    pub max_date: Option<String>,
    pub top_values: Vec<ValueCount>,
}

impl RawColumnProfile {
    pub fn null_ratio(&self) -> f64 {
        if self.row_count == 0 {
            0.0
        } else {
            self.null_count as f64 / self.row_count as f64
        }
    }

    pub fn distinct_ratio(&self) -> f64 {
        let non_null = self.row_count.saturating_sub(self.null_count);
        if non_null == 0 {
            0.0
        } else {
            self.distinct_count as f64 / non_null as f64
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValueCount {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DistinctSet {
    /// Canonical value → frequency among non-null rows.
    pub counts: indexmap::IndexMap<String, u64>,
    pub non_null_count: u64,
    pub truncated: bool,
}

impl DistinctSet {
    pub fn coverage_of(&self, haystack: &DistinctSet) -> f64 {
        if self.non_null_count == 0 {
            return 0.0;
        }
        let mut matched = 0u64;
        for (value, freq) in &self.counts {
            if haystack.counts.contains_key(value) {
                matched += *freq;
            }
        }
        matched as f64 / self.non_null_count as f64
    }
}

pub fn overlap_value_limit() -> usize {
    MAX_DISTINCT_FOR_OVERLAP
}

pub fn quote_ident(name: &str) -> Result<String> {
    crate::query::sql::quote_ident(name)
}

/// Column-aligned numeric sample. `summed` marks a grain aggregate, not a stored column.
#[derive(Debug, Clone)]
pub struct NumericSample {
    pub label: String,
    pub columns: Vec<NumericColumn>,
}

#[derive(Debug, Clone)]
pub struct NumericColumn {
    pub table: String,
    pub column: String,
    pub summed: bool,
    pub values: Vec<Option<f64>>,
}

impl NumericSample {
    pub fn n_rows(&self) -> usize {
        self.columns.first().map(|c| c.values.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty() || self.n_rows() == 0
    }
}

pub fn sample_row_indices(n: usize, max_rows: usize) -> Vec<usize> {
    if n == 0 || max_rows == 0 {
        return Vec::new();
    }
    let take = n.min(max_rows);
    if take == n {
        return (0..n).collect();
    }
    (0..take).map(|i| i * n / take).collect()
}

pub fn parse_opt_f64(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed == "\\N" {
        None
    } else {
        trimmed.parse().ok()
    }
}

pub fn numeric_sample_from_query(
    label: impl Into<String>,
    columns: Vec<NumericColumnSpec>,
    rows: &[Vec<String>],
) -> NumericSample {
    let mut out: Vec<NumericColumn> = columns
        .into_iter()
        .map(|spec| NumericColumn {
            table: spec.table,
            column: spec.column,
            summed: spec.summed,
            values: Vec::with_capacity(rows.len()),
        })
        .collect();
    for row in rows {
        for (i, col) in out.iter_mut().enumerate() {
            let raw = row.get(i).map(String::as_str).unwrap_or("");
            col.values.push(parse_opt_f64(raw));
        }
    }
    NumericSample {
        label: label.into(),
        columns: out,
    }
}

#[derive(Debug, Clone)]
pub struct NumericColumnSpec {
    pub table: String,
    pub column: String,
    pub summed: bool,
}
