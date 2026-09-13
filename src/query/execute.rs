use std::collections::HashMap;

use indexmap::IndexMap;

use crate::error::{AnalyticsError, Result};
use crate::kpi::Aggregation;
use crate::query::{FilterOp, QueryPlan, QueryResult, SelectKind};
use crate::types::{Cell, ColumnRef, DataType};

#[derive(Debug, Clone)]
pub struct MemoryTable {
    pub name: String,
    pub columns: Vec<String>,
    pub types: Vec<DataType>,
    pub rows: Vec<Vec<Cell>>,
}

impl MemoryTable {
    pub fn row_count(&self) -> u64 {
        self.rows.len() as u64
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

pub fn execute_plan_in_memory(
    tables: &IndexMap<String, MemoryTable>,
    plan: &QueryPlan,
) -> Result<QueryResult> {
    let mut working = load_table(tables, &plan.from)?;
    for join in &plan.joins {
        let right = load_table(tables, &join.table)?;
        working = hash_join(working, right, &join.left, &join.right)?;
    }

    working.rows.retain(|row| row_matches(&working.columns, row, plan));

    let aliases: Vec<String> = plan.select.iter().map(|s| sanitize(&s.alias)).collect();
    let grouped = !plan.group_by.is_empty()
        || plan.select.iter().any(|s| matches!(s.expr, SelectKind::Aggregate { .. }));

    let mut out_rows = Vec::new();
    if grouped {
        let mut groups: IndexMap<String, Vec<Vec<Cell>>> = IndexMap::new();
        for row in working.rows {
            let key = group_key(&working.columns, &row, &plan.group_by)?;
            groups.entry(key).or_default().push(row);
        }
        for rows in groups.into_values() {
            out_rows.push(eval_select(&working.columns, &rows, plan)?);
        }
    } else {
        for row in working.rows {
            out_rows.push(eval_select(&working.columns, std::slice::from_ref(&row), plan)?);
        }
    }

    if !plan.order_by.is_empty() {
        let idxs: Vec<(usize, bool)> = plan
            .order_by
            .iter()
            .filter_map(|(field, desc)| {
                aliases
                    .iter()
                    .position(|a| a == &sanitize(field) || a == field)
                    .map(|i| (i, *desc))
            })
            .collect();
        out_rows.sort_by(|a, b| {
            for (idx, desc) in &idxs {
                let ord = cmp_cell(&a[*idx], &b[*idx]);
                if ord != std::cmp::Ordering::Equal {
                    return if *desc { ord.reverse() } else { ord };
                }
            }
            std::cmp::Ordering::Equal
        });
    }
    if let Some(limit) = plan.limit {
        out_rows.truncate(limit);
    }

    Ok(QueryResult {
        columns: aliases,
        rows: out_rows
            .into_iter()
            .map(|r| r.into_iter().map(|c| c.display()).collect())
            .collect(),
    })
}

struct WorkingTable {
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

fn load_table(tables: &IndexMap<String, MemoryTable>, name: &str) -> Result<WorkingTable> {
    let table = tables
        .get(name)
        .ok_or_else(|| AnalyticsError::UnknownTable(name.to_string()))?;
    Ok(WorkingTable {
        columns: table
            .columns
            .iter()
            .map(|c| format!("{name}.{c}"))
            .collect(),
        rows: table.rows.clone(),
    })
}

fn hash_join(
    left: WorkingTable,
    right: WorkingTable,
    left_col: &ColumnRef,
    right_col: &ColumnRef,
) -> Result<WorkingTable> {
    let lidx = col_index(&left.columns, left_col)?;
    let ridx = col_index(&right.columns, right_col)?;
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, row) in right.rows.iter().enumerate() {
        if row[ridx].is_null() {
            continue;
        }
        index.entry(row[ridx].canonical()).or_default().push(i);
    }
    let mut rows = Vec::new();
    for lrow in left.rows {
        if lrow[lidx].is_null() {
            continue;
        }
        if let Some(matches) = index.get(&lrow[lidx].canonical()) {
            for ri in matches {
                let mut joined = lrow.clone();
                joined.extend(right.rows[*ri].clone());
                rows.push(joined);
            }
        }
    }
    let mut columns = left.columns;
    columns.extend(right.columns);
    Ok(WorkingTable { columns, rows })
}

fn col_index(columns: &[String], col: &ColumnRef) -> Result<usize> {
    let qualified = col.qualified();
    columns
        .iter()
        .position(|c| c == &qualified || c.ends_with(&format!(".{}", col.column)))
        .ok_or_else(|| AnalyticsError::QueryPlan(format!("column `{qualified}` missing in working set")))
}

fn row_matches(columns: &[String], row: &[Cell], plan: &QueryPlan) -> bool {
    plan.filters.iter().all(|f| {
        let Ok(idx) = col_index(columns, &f.column) else {
            return false;
        };
        apply_filter(&row[idx], f.operator, &f.value)
    })
}

fn apply_filter(cell: &Cell, op: FilterOp, value: &serde_json::Value) -> bool {
    if cell.is_null() {
        return false;
    }
    match op {
        FilterOp::YearEquals => cell
            .as_date()
            .map(|d| json_i64(value) == Some(d.format("%Y").to_string().parse::<i64>().unwrap_or(0)))
            .unwrap_or(false),
        FilterOp::MonthEquals => cell
            .as_date()
            .map(|d| json_i64(value) == Some(d.format("%m").to_string().parse::<i64>().unwrap_or(0)))
            .unwrap_or(false),
        FilterOp::Eq => cell.canonical() == json_text(value),
        FilterOp::Neq => cell.canonical() != json_text(value),
        FilterOp::Gt => cmp_with(cell, value) == Some(std::cmp::Ordering::Greater),
        FilterOp::Gte => matches!(
            cmp_with(cell, value),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
        FilterOp::Lt => cmp_with(cell, value) == Some(std::cmp::Ordering::Less),
        FilterOp::Lte | FilterOp::OnOrBefore => matches!(
            cmp_with(cell, value),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        FilterOp::OnOrAfter => matches!(
            cmp_with(cell, value),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
    }
}

fn json_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn json_i64(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn cmp_with(cell: &Cell, value: &serde_json::Value) -> Option<std::cmp::Ordering> {
    if let (Some(a), Some(b)) = (cell.as_f64(), json_i64(value).map(|n| n as f64).or_else(|| {
        value.as_f64().or_else(|| json_text(value).parse().ok())
    })) {
        return a.partial_cmp(&b);
    }
    Some(cell.canonical().cmp(&json_text(value)))
}

fn group_key(columns: &[String], row: &[Cell], group_by: &[ColumnRef]) -> Result<String> {
    let mut parts = Vec::new();
    for col in group_by {
        let idx = col_index(columns, col)?;
        parts.push(row[idx].canonical());
    }
    Ok(parts.join("||"))
}

fn eval_select(columns: &[String], rows: &[Vec<Cell>], plan: &QueryPlan) -> Result<Vec<Cell>> {
    let mut out = Vec::new();
    for expr in &plan.select {
        match &expr.expr {
            SelectKind::Column(col) => {
                let idx = col_index(columns, col)?;
                out.push(rows[0][idx].clone());
            }
            SelectKind::Aggregate { func, column } => {
                out.push(aggregate(columns, rows, *func, column.as_ref())?);
            }
        }
    }
    Ok(out)
}

fn aggregate(
    columns: &[String],
    rows: &[Vec<Cell>],
    func: Aggregation,
    column: Option<&ColumnRef>,
) -> Result<Cell> {
    match func {
        Aggregation::Count => Ok(Cell::Int(rows.len() as i64)),
        Aggregation::CountDistinct => {
            let col = column.ok_or_else(|| AnalyticsError::QueryPlan("COUNT DISTINCT needs a column".into()))?;
            let idx = col_index(columns, col)?;
            let mut set = std::collections::HashSet::new();
            for row in rows {
                if !row[idx].is_null() {
                    set.insert(row[idx].canonical());
                }
            }
            Ok(Cell::Int(set.len() as i64))
        }
        Aggregation::Sum | Aggregation::Avg | Aggregation::Min | Aggregation::Max => {
            let col = column.ok_or_else(|| AnalyticsError::QueryPlan("aggregate needs a column".into()))?;
            let idx = col_index(columns, col)?;
            let nums: Vec<f64> = rows.iter().filter_map(|r| r[idx].as_f64()).collect();
            if nums.is_empty() {
                return Ok(Cell::Null);
            }
            let value = match func {
                Aggregation::Sum => nums.iter().sum(),
                Aggregation::Avg => nums.iter().sum::<f64>() / nums.len() as f64,
                Aggregation::Min => nums.iter().cloned().fold(f64::INFINITY, f64::min),
                Aggregation::Max => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                _ => unreachable!(),
            };
            Ok(Cell::Float(value))
        }
    }
}

fn sanitize(alias: &str) -> String {
    alias
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn cmp_cell(a: &Cell, b: &Cell) -> std::cmp::Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.canonical().cmp(&b.canonical()),
    }
}
