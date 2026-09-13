use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::connector::{
    DistinctSet, NumericColumn, NumericColumnSpec, NumericSample, RawColumnProfile,
    RawDatabaseProfile, RawTableProfile, ValueCount,
};
use crate::error::{AnalyticsError, Result};
use crate::limits::{
    MAX_FIXTURE_ROWS, MAX_FIXTURE_TABLES, MEDIAN_VALUE_LIMIT, TOP_VALUES,
};
use crate::query::{execute_plan_in_memory, MemoryTable, QueryPlan, QueryResult};
use crate::schema::{DatabaseSchema, FixtureSchemaFile};
use crate::types::{parse_cell, Cell, DataType, SqlDialect};

use super::DataSource;

const SKIP_FILES: &[&str] = &["schema.json", "ground_truth.json", "readme.md"];

pub struct FixtureConnector {
    name: String,
    root: PathBuf,
    db_schema: DatabaseSchema,
    tables: IndexMap<String, MemoryTable>,
}

impl FixtureConnector {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(AnalyticsError::fixture(
                &root,
                "path is not a directory",
            ));
        }
        let schema_path = root.join("schema.json");
        let schema_text = fs::read_to_string(&schema_path).map_err(|e| {
            AnalyticsError::fixture(&schema_path, format!("cannot read schema.json: {e}"))
        })?;
        let file: FixtureSchemaFile = serde_json::from_str(&schema_text)?;
        let fallback = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let mut schema = file.into_schema(fallback);

        if schema.tables.len() > MAX_FIXTURE_TABLES {
            return Err(AnalyticsError::FixtureBudget(format!(
                "{} tables found, V1 budget is {MAX_FIXTURE_TABLES}. Add another small fixture instead of growing this one.",
                schema.tables.len()
            )));
        }

        let mut tables = IndexMap::new();
        let mut total_rows = 0u64;
        for table in &schema.tables {
            let (file_path, format) = find_table_file(&root, &table.name)?;
            let memory = match format {
                TableFileFormat::Csv => load_csv(&file_path, table)?,
                TableFileFormat::Json => load_json(&file_path, table)?,
                TableFileFormat::Parquet => {
                    return Err(AnalyticsError::fixture(
                        file_path,
                        "Parquet fixtures are recognized but not enabled in V1. Convert the table to CSV or JSON.",
                    ));
                }
            };
            total_rows += memory.row_count();
            if total_rows > MAX_FIXTURE_ROWS {
                return Err(AnalyticsError::FixtureBudget(format!(
                    "loading `{}` pushed the fixture to {total_rows} rows (V1 budget is {MAX_FIXTURE_ROWS}). Do not scale this fixture; split into another small model instead.",
                    table.name
                )));
            }
            tables.insert(table.name.clone(), memory);
        }

        for table in &mut schema.tables {
            if let Some(mem) = tables.get(&table.name) {
                table.approximate_row_count = Some(mem.row_count());
            }
        }

        Ok(Self {
            name: schema.name.clone(),
            root,
            db_schema: schema,
            tables,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn memory_tables(&self) -> &IndexMap<String, MemoryTable> {
        &self.tables
    }

    pub fn row_count_total(&self) -> u64 {
        self.tables.values().map(|t| t.row_count()).sum()
    }
}

impl DataSource for FixtureConnector {
    fn name(&self) -> &str {
        &self.name
    }

    fn dialect(&self) -> SqlDialect {
        SqlDialect::Generic
    }

    fn schema(&self) -> Result<DatabaseSchema> {
        Ok(self.db_schema.clone())
    }

    fn profile_raw(&self) -> Result<RawDatabaseProfile> {
        let mut tables = Vec::new();
        for table_schema in &self.db_schema.tables {
            let mem = self
                .tables
                .get(&table_schema.name)
                .ok_or_else(|| AnalyticsError::UnknownTable(table_schema.name.clone()))?;
            tables.push(profile_memory_table(table_schema, mem));
        }
        Ok(RawDatabaseProfile { tables })
    }

    fn distinct_values(
        &self,
        table: &str,
        column: &str,
        max_values: usize,
    ) -> Result<DistinctSet> {
        let mem = self
            .tables
            .get(table)
            .ok_or_else(|| AnalyticsError::UnknownTable(table.to_string()))?;
        let idx = mem
            .column_index(column)
            .ok_or_else(|| AnalyticsError::UnknownColumn {
                table: table.to_string(),
                column: column.to_string(),
            })?;
        let mut counts: IndexMap<String, u64> = IndexMap::new();
        let mut non_null = 0u64;
        let mut truncated = false;
        for row in &mem.rows {
            if row[idx].is_null() {
                continue;
            }
            non_null += 1;
            let key = row[idx].canonical();
            if let Some(freq) = counts.get_mut(&key) {
                *freq += 1;
            } else if counts.len() < max_values {
                counts.insert(key, 1);
            } else {
                truncated = true;
            }
        }
        Ok(DistinctSet {
            counts,
            non_null_count: non_null,
            truncated,
        })
    }

    fn execute_sql(&self, sql: &str) -> Result<QueryResult> {
        Err(AnalyticsError::msg(format!(
            "the fixture connector executes planned queries in-process; it does not parse arbitrary SQL. Plan SQL was:\n{sql}"
        )))
    }

    fn execute_plan(&self, plan: &QueryPlan) -> Result<QueryResult> {
        execute_plan_in_memory(&self.tables, plan)
    }

    fn sample_numeric(&self, table: &str, columns: &[String], max_rows: usize) -> Result<NumericSample> {
        let mem = self
            .tables
            .get(table)
            .ok_or_else(|| AnalyticsError::UnknownTable(table.to_string()))?;
        sample_memory_numeric(table, mem, columns, max_rows)
    }

    fn sample_join_numeric(
        &self,
        left_table: &str,
        left_columns: &[String],
        right_table: &str,
        right_columns: &[String],
        left_key: &str,
        right_key: &str,
        max_rows: usize,
    ) -> Result<NumericSample> {
        let left = self
            .tables
            .get(left_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(left_table.to_string()))?;
        let right = self
            .tables
            .get(right_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(right_table.to_string()))?;
        let lkey = left.column_index(left_key).ok_or_else(|| AnalyticsError::UnknownColumn {
            table: left_table.to_string(),
            column: left_key.to_string(),
        })?;
        let rkey = right
            .column_index(right_key)
            .ok_or_else(|| AnalyticsError::UnknownColumn {
                table: right_table.to_string(),
                column: right_key.to_string(),
            })?;

        let mut parent: HashMap<String, usize> = HashMap::new();
        for (i, row) in right.rows.iter().enumerate() {
            if row[rkey].is_null() {
                continue;
            }
            parent.entry(row[rkey].canonical()).or_insert(i);
        }

        let left_idx = column_indexes(left, left_table, left_columns)?;
        let right_idx = column_indexes(right, right_table, right_columns)?;
        let indices = crate::connector::sample_row_indices(left.rows.len(), max_rows);

        let mut specs = Vec::new();
        for name in left_columns {
            specs.push(NumericColumnSpec {
                table: left_table.to_string(),
                column: name.clone(),
                summed: false,
            });
        }
        for name in right_columns {
            specs.push(NumericColumnSpec {
                table: right_table.to_string(),
                column: name.clone(),
                summed: false,
            });
        }
        let mut out: Vec<NumericColumn> = specs
            .into_iter()
            .map(|spec| NumericColumn {
                table: spec.table,
                column: spec.column,
                summed: spec.summed,
                values: Vec::new(),
            })
            .collect();

        for i in indices {
            let lrow = &left.rows[i];
            if lrow[lkey].is_null() {
                continue;
            }
            let Some(&ri) = parent.get(&lrow[lkey].canonical()) else {
                continue;
            };
            let rrow = &right.rows[ri];
            for (slot, idx) in left_idx.iter().enumerate() {
                out[slot].values.push(lrow[*idx].as_f64());
            }
            for (j, idx) in right_idx.iter().enumerate() {
                out[left_idx.len() + j].values.push(rrow[*idx].as_f64());
            }
        }

        Ok(NumericSample {
            label: format!("{left_table}⋈{right_table}"),
            columns: out,
        })
    }

    fn sample_grouped_sum(
        &self,
        child_table: &str,
        child_key: &str,
        child_measure: &str,
        parent_table: &str,
        parent_key: &str,
        parent_measures: &[String],
        max_groups: usize,
    ) -> Result<NumericSample> {
        let child = self
            .tables
            .get(child_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(child_table.to_string()))?;
        let parent = self
            .tables
            .get(parent_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(parent_table.to_string()))?;
        let ck = child
            .column_index(child_key)
            .ok_or_else(|| AnalyticsError::UnknownColumn {
                table: child_table.to_string(),
                column: child_key.to_string(),
            })?;
        let cm = child
            .column_index(child_measure)
            .ok_or_else(|| AnalyticsError::UnknownColumn {
                table: child_table.to_string(),
                column: child_measure.to_string(),
            })?;
        let pk = parent
            .column_index(parent_key)
            .ok_or_else(|| AnalyticsError::UnknownColumn {
                table: parent_table.to_string(),
                column: parent_key.to_string(),
            })?;
        let parent_idx = column_indexes(parent, parent_table, parent_measures)?;

        let mut sums: HashMap<String, f64> = HashMap::new();
        for row in &child.rows {
            if row[ck].is_null() {
                continue;
            }
            if let Some(v) = row[cm].as_f64() {
                *sums.entry(row[ck].canonical()).or_insert(0.0) += v;
            }
        }

        let indices = crate::connector::sample_row_indices(parent.rows.len(), max_groups);
        let mut columns = Vec::new();
        columns.push(NumericColumn {
            table: child_table.to_string(),
            column: child_measure.to_string(),
            summed: true,
            values: Vec::new(),
        });
        for name in parent_measures {
            columns.push(NumericColumn {
                table: parent_table.to_string(),
                column: name.clone(),
                summed: false,
                values: Vec::new(),
            });
        }

        for i in indices {
            let prow = &parent.rows[i];
            if prow[pk].is_null() {
                continue;
            }
            let key = prow[pk].canonical();
            columns[0].values.push(Some(*sums.get(&key).unwrap_or(&0.0)));
            for (j, idx) in parent_idx.iter().enumerate() {
                columns[j + 1].values.push(prow[*idx].as_f64());
            }
        }

        Ok(NumericSample {
            label: format!("sum({child_table}.{child_measure})@{parent_table}"),
            columns,
        })
    }
}

enum TableFileFormat {
    Csv,
    Json,
    Parquet,
}

fn column_indexes(table: &MemoryTable, table_name: &str, columns: &[String]) -> Result<Vec<usize>> {
    let mut idxs = Vec::with_capacity(columns.len());
    for name in columns {
        let idx = table.column_index(name).ok_or_else(|| AnalyticsError::UnknownColumn {
            table: table_name.to_string(),
            column: name.clone(),
        })?;
        idxs.push(idx);
    }
    Ok(idxs)
}

fn sample_memory_numeric(
    table_name: &str,
    mem: &MemoryTable,
    columns: &[String],
    max_rows: usize,
) -> Result<NumericSample> {
    let idxs = column_indexes(mem, table_name, columns)?;
    let indices = crate::connector::sample_row_indices(mem.rows.len(), max_rows);
    let mut out: Vec<NumericColumn> = columns
        .iter()
        .map(|name| NumericColumn {
            table: table_name.to_string(),
            column: name.clone(),
            summed: false,
            values: Vec::with_capacity(indices.len()),
        })
        .collect();
    for i in indices {
        let row = &mem.rows[i];
        for (slot, idx) in idxs.iter().enumerate() {
            out[slot].values.push(row[*idx].as_f64());
        }
    }
    Ok(NumericSample {
        label: table_name.to_string(),
        columns: out,
    })
}

fn find_table_file(root: &Path, table: &str) -> Result<(PathBuf, TableFileFormat)> {
    let csv = root.join(format!("{table}.csv"));
    if csv.exists() {
        return Ok((csv, TableFileFormat::Csv));
    }
    let json = root.join(format!("{table}.json"));
    if json.exists() {
        return Ok((json, TableFileFormat::Json));
    }
    let parquet = root.join(format!("{table}.parquet"));
    if parquet.exists() {
        return Ok((parquet, TableFileFormat::Parquet));
    }
    Err(AnalyticsError::fixture(
        root,
        format!("no CSV/JSON/Parquet file found for table `{table}`"),
    ))
}

fn load_csv(path: &Path, table: &crate::schema::TableSchema) -> Result<MemoryTable> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let headers: Vec<String> = reader.headers()?.iter().map(|s| s.to_string()).collect();
    let col_types: Vec<DataType> = table.columns.iter().map(|c| c.data_type.clone()).collect();
    let col_names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
    let header_index: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let mut row = Vec::with_capacity(col_names.len());
        for (name, ty) in col_names.iter().zip(col_types.iter()) {
            let value = header_index
                .get(name)
                .and_then(|i| record.get(*i))
                .unwrap_or("");
            row.push(parse_cell(value, ty));
        }
        rows.push(row);
    }
    Ok(MemoryTable {
        name: table.name.clone(),
        columns: col_names,
        types: col_types,
        rows,
    })
}

fn load_json(path: &Path, table: &crate::schema::TableSchema) -> Result<MemoryTable> {
    let text = fs::read_to_string(path)?;
    let records: Vec<serde_json::Value> = serde_json::from_str(&text)?;
    let col_names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
    let col_types: Vec<DataType> = table.columns.iter().map(|c| c.data_type.clone()).collect();
    let mut rows = Vec::new();
    for record in records {
        let obj = record.as_object().ok_or_else(|| {
            AnalyticsError::fixture(path, "JSON table must be an array of objects")
        })?;
        let mut row = Vec::with_capacity(col_names.len());
        for (name, ty) in col_names.iter().zip(col_types.iter()) {
            let raw = match obj.get(name) {
                None | Some(serde_json::Value::Null) => String::new(),
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
            };
            row.push(parse_cell(&raw, ty));
        }
        rows.push(row);
    }
    Ok(MemoryTable {
        name: table.name.clone(),
        columns: col_names,
        types: col_types,
        rows,
    })
}

fn profile_memory_table(
    schema: &crate::schema::TableSchema,
    table: &MemoryTable,
) -> RawTableProfile {
    let row_count = table.row_count();
    let mut columns = Vec::new();
    for (idx, col) in schema.columns.iter().enumerate() {
        columns.push(profile_column(col, row_count, table.rows.iter().map(|r| &r[idx])));
    }
    RawTableProfile {
        table: schema.name.clone(),
        row_count,
        columns,
    }
}

fn profile_column<'a>(
    col: &crate::schema::ColumnSchema,
    row_count: u64,
    values: impl Iterator<Item = &'a Cell>,
) -> RawColumnProfile {
    let mut null_count = 0u64;
    let mut distinct: IndexMap<String, u64> = IndexMap::new();
    let mut numeric: Vec<f64> = Vec::new();
    let mut length_sum = 0u64;
    let mut length_n = 0u64;
    let mut min_s: Option<String> = None;
    let mut max_s: Option<String> = None;
    let mut min_date: Option<String> = None;
    let mut max_date: Option<String> = None;

    for cell in values {
        if cell.is_null() {
            null_count += 1;
            continue;
        }
        let canon = cell.canonical();
        *distinct.entry(canon.clone()).or_insert(0) += 1;
        if min_s.as_ref().map(|m| canon < *m).unwrap_or(true) {
            min_s = Some(canon.clone());
        }
        if max_s.as_ref().map(|m| canon > *m).unwrap_or(true) {
            max_s = Some(canon.clone());
        }
        if let Some(n) = cell.as_f64() {
            if numeric.len() < MEDIAN_VALUE_LIMIT {
                numeric.push(n);
            }
        }
        if matches!(col.data_type, DataType::Text | DataType::Json) {
            length_sum += canon.len() as u64;
            length_n += 1;
        }
        if let Some(d) = cell.as_date() {
            let ds = d.to_string();
            if min_date.as_ref().map(|m| ds < *m).unwrap_or(true) {
                min_date = Some(ds.clone());
            }
            if max_date.as_ref().map(|m| ds > *m).unwrap_or(true) {
                max_date = Some(ds);
            }
        }
    }

    let mut top: Vec<ValueCount> = distinct
        .iter()
        .map(|(value, count)| ValueCount {
            value: value.clone(),
            count: *count,
        })
        .collect();
    top.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    top.truncate(TOP_VALUES);

    let (avg, stddev, median) = numeric_stats(&numeric);

    RawColumnProfile {
        column: col.name.clone(),
        row_count,
        null_count,
        distinct_count: distinct.len() as u64,
        min: min_s,
        max: max_s,
        avg,
        stddev,
        median,
        avg_length: if length_n == 0 {
            None
        } else {
            Some(length_sum as f64 / length_n as f64)
        },
        min_date,
        max_date,
        top_values: top,
    }
}

fn numeric_stats(values: &[f64]) -> (Option<f64>, Option<f64>, Option<f64>) {
    if values.is_empty() {
        return (None, None, None);
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| {
        let d = v - mean;
        d * d
    }).sum::<f64>() / n;
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        let hi = sorted.len() / 2;
        (sorted[hi - 1] + sorted[hi]) / 2.0
    };
    (Some(mean), Some(var.sqrt()), Some(median))
}

#[allow(dead_code)]
fn skip_file_name(name: &str) -> bool {
    SKIP_FILES.iter().any(|s| s.eq_ignore_ascii_case(name))
}
