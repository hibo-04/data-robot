use crate::error::{AnalyticsError, Result};
use crate::kpi::Aggregation;
use crate::query::{FilterOp, PlannedFilter, QueryPlan, SelectKind};
use crate::types::{ColumnRef, SqlDialect};

pub fn quote_ident(name: &str) -> Result<String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(AnalyticsError::QueryPlan(format!(
            "refusing to interpolate identifier `{name}`"
        )));
    }
    Ok(format!("\"{name}\""))
}

pub fn render(plan: &QueryPlan, _dialect: SqlDialect) -> Result<String> {
    let mut sql = String::from("SELECT\n");
    let mut parts = Vec::new();
    for expr in &plan.select {
        parts.push(format!("    {} AS {}", render_select(&expr.expr)?, quote_ident(&sanitize_alias(&expr.alias))?));
    }
    sql.push_str(&parts.join(",\n"));
    sql.push_str(&format!("\nFROM {}\n", quote_ident(&plan.from)?));
    for join in &plan.joins {
        sql.push_str(&format!(
            "JOIN {} ON {} = {}\n",
            quote_ident(&join.table)?,
            qcol(&join.left)?,
            qcol(&join.right)?
        ));
    }
    if !plan.filters.is_empty() {
        sql.push_str("WHERE\n");
        let filters: Result<Vec<_>> = plan.filters.iter().map(render_filter).collect();
        sql.push_str(&filters?.join("\nAND\n"));
        sql.push('\n');
    }
    if !plan.group_by.is_empty() {
        sql.push_str("GROUP BY ");
        let groups: Result<Vec<_>> = plan.group_by.iter().map(qcol).collect();
        sql.push_str(&groups?.join(", "));
        sql.push('\n');
    }
    if !plan.order_by.is_empty() {
        sql.push_str("ORDER BY ");
        let mut orders = Vec::new();
        for (field, desc) in &plan.order_by {
            let dir = if *desc { "DESC" } else { "ASC" };
            orders.push(format!("{} {dir}", quote_ident(&sanitize_alias(field))?));
        }
        sql.push_str(&orders.join(", "));
        sql.push('\n');
    }
    if let Some(limit) = plan.limit {
        sql.push_str(&format!("LIMIT {limit}\n"));
    }
    Ok(sql)
}

fn sanitize_alias(alias: &str) -> String {
    alias
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn qcol(col: &ColumnRef) -> Result<String> {
    Ok(format!("{}.{}", quote_ident(&col.table)?, quote_ident(&col.column)?))
}

fn render_select(kind: &SelectKind) -> Result<String> {
    match kind {
        SelectKind::Column(col) => qcol(col),
        SelectKind::Aggregate { func, column } => {
            let inner = match column {
                Some(col) => qcol(col)?,
                None => "*".to_string(),
            };
            if *func == Aggregation::CountDistinct {
                Ok(format!("COUNT(DISTINCT {inner})"))
            } else {
                Ok(format!("{}({inner})", func.sql_fn()))
            }
        }
    }
}

fn render_filter(filter: &PlannedFilter) -> Result<String> {
    let col = qcol(&filter.column)?;
    let literal = json_literal(&filter.value)?;
    let expr = match filter.operator {
        FilterOp::Eq => format!("{col} = {literal}"),
        FilterOp::Neq => format!("{col} <> {literal}"),
        FilterOp::Gt => format!("{col} > {literal}"),
        FilterOp::Gte => format!("{col} >= {literal}"),
        FilterOp::Lt => format!("{col} < {literal}"),
        FilterOp::Lte => format!("{col} <= {literal}"),
        FilterOp::YearEquals => format!("EXTRACT(YEAR FROM {col}) = {literal}"),
        FilterOp::MonthEquals => format!("EXTRACT(MONTH FROM {col}) = {literal}"),
        FilterOp::OnOrAfter => format!("{col} >= {literal}"),
        FilterOp::OnOrBefore => format!("{col} <= {literal}"),
    };
    Ok(format!("    {expr}"))
}

fn json_literal(value: &serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::Null => Ok("NULL".to_string()),
        serde_json::Value::Bool(v) => Ok(if *v { "TRUE".into() } else { "FALSE".into() }),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => {
            if s.chars().any(|c| c == '\'' || c == ';') {
                return Err(AnalyticsError::QueryPlan(
                    "filter string contains unsupported characters".into(),
                ));
            }
            Ok(format!("'{s}'"))
        }
        _ => Err(AnalyticsError::QueryPlan(
            "unsupported filter value".into(),
        )),
    }
}
