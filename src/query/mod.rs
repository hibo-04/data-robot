use serde::{Deserialize, Serialize};

use crate::error::{AnalyticsError, Result};
use crate::kpi::{Aggregation, KpiCandidate};
use crate::semantic::SemanticModel;
use crate::types::{ColumnRef, SqlDialect};

pub mod execute;
pub mod sql;

pub use execute::{execute_plan_in_memory, MemoryTable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticQuery {
    pub measures: Vec<String>,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default)]
    pub filters: Vec<QueryFilter>,
    #[serde(default)]
    pub order_by: Option<QueryOrder>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOp,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    YearEquals,
    MonthEquals,
    OnOrAfter,
    OnOrBefore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOrder {
    pub field: String,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub select: Vec<SelectExpr>,
    pub from: String,
    pub joins: Vec<JoinStep>,
    pub filters: Vec<PlannedFilter>,
    pub group_by: Vec<ColumnRef>,
    pub order_by: Vec<(String, bool)>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectExpr {
    pub expr: SelectKind,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectKind {
    Column(ColumnRef),
    Aggregate {
        func: Aggregation,
        column: Option<ColumnRef>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinStep {
    pub table: String,
    pub left: ColumnRef,
    pub right: ColumnRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFilter {
    pub column: ColumnRef,
    pub operator: FilterOp,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl QueryPlan {
    pub fn to_sql(&self, dialect: SqlDialect) -> Result<String> {
        sql::render(self, dialect)
    }
}

pub fn plan_query(
    model: &SemanticModel,
    kpis: &[KpiCandidate],
    query: &SemanticQuery,
) -> Result<QueryPlan> {
    if query.measures.is_empty() {
        return Err(AnalyticsError::QueryPlan(
            "semantic query needs at least one measure".to_string(),
        ));
    }

    let mut select = Vec::new();
    let mut tables: Vec<String> = Vec::new();
    let mut group_by = Vec::new();

    for dim in &query.dimensions {
        let col = resolve_dimension(model, dim)?;
        if !tables.iter().any(|t| t == &col.table) {
            tables.push(col.table.clone());
        }
        select.push(SelectExpr {
            alias: dim.clone(),
            expr: SelectKind::Column(col.clone()),
        });
        group_by.push(col);
    }

    for measure in &query.measures {
        let (alias, kind, table) = resolve_measure(model, kpis, measure)?;
        if !tables.iter().any(|t| t == &table) {
            tables.push(table);
        }
        select.push(SelectExpr { alias, expr: kind });
    }

    let mut filters = Vec::new();
    for filter in &query.filters {
        let col = resolve_field(model, &filter.field)?;
        if !tables.iter().any(|t| t == &col.table) {
            tables.push(col.table.clone());
        }
        filters.push(PlannedFilter {
            column: col,
            operator: filter.operator,
            value: filter.value.clone(),
        });
    }

    let fact = choose_fact_table(kpis, &query.measures, &tables)?;
    let joins = join_tree(model, &fact, &tables)?;

    let mut order_by = Vec::new();
    if let Some(order) = &query.order_by {
        order_by.push((order.field.clone(), order.descending));
    } else if let Some(first_measure) = query.measures.first() {
        order_by.push((first_measure.clone(), true));
    }

    Ok(QueryPlan {
        select,
        from: fact,
        joins,
        filters,
        group_by,
        order_by,
        limit: query.limit,
    })
}

fn resolve_dimension(model: &SemanticModel, name: &str) -> Result<ColumnRef> {
    resolve_field(model, name)
}

fn resolve_field(model: &SemanticModel, name: &str) -> Result<ColumnRef> {
    if let Some((table, column)) = name.split_once('.') {
        if model.column(table, column).is_some() {
            return Ok(ColumnRef::new(table, column));
        }
    }
    if let Some(col) = model.find_by_name(name) {
        return Ok(col.ref_.clone());
    }
    let matches: Vec<_> = model
        .columns
        .iter()
        .filter(|c| c.ref_.column.eq_ignore_ascii_case(name))
        .collect();
    if matches.len() == 1 {
        return Ok(matches[0].ref_.clone());
    }
    Err(AnalyticsError::QueryPlan(format!(
        "cannot resolve field `{name}` against the semantic model"
    )))
}

fn resolve_measure(
    model: &SemanticModel,
    kpis: &[KpiCandidate],
    name: &str,
) -> Result<(String, SelectKind, String)> {
    if let Some(kpi) = kpis.iter().find(|k| k.matches_name(name)) {
        if kpi.aggregation.is_none() || kpi.source.is_none() {
            return Err(AnalyticsError::QueryPlan(format!(
                "KPI `{name}` is derived ({:?}) and cannot be queried directly in V1; query its inputs instead",
                kpi.expression
            )));
        }
        let source = kpi.source.clone().unwrap();
        let func = kpi.aggregation.unwrap();
        return Ok((
            alias_from(kpi.display_name()),
            SelectKind::Aggregate {
                func,
                column: Some(source.clone()),
            },
            source.table,
        ));
    }
    let col = resolve_field(model, name)?;
    Ok((
        name.to_string(),
        SelectKind::Aggregate {
            func: Aggregation::Sum,
            column: Some(col.clone()),
        },
        col.table,
    ))
}

fn alias_from(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn choose_fact_table(
    kpis: &[KpiCandidate],
    measures: &[String],
    tables: &[String],
) -> Result<String> {
    for m in measures {
        if let Some(kpi) = kpis.iter().find(|k| k.matches_name(m)) {
            if let Some(src) = &kpi.source {
                return Ok(src.table.clone());
            }
        }
    }
    tables
        .first()
        .cloned()
        .ok_or_else(|| AnalyticsError::QueryPlan("no tables involved in query".into()))
}

fn join_tree(model: &SemanticModel, fact: &str, tables: &[String]) -> Result<Vec<JoinStep>> {
    let mut needed: Vec<String> = tables
        .iter()
        .filter(|t| t.as_str() != fact)
        .cloned()
        .collect();
    needed.sort();
    needed.dedup();
    if needed.is_empty() {
        return Ok(Vec::new());
    }

    let mut joins = Vec::new();
    let mut reached = vec![fact.to_string()];
    for target in needed {
        let path = shortest_path(model, &reached, &target)?;
        for step in path {
            if !reached.iter().any(|t| t == &step.table) {
                reached.push(step.table.clone());
                joins.push(step);
            }
        }
    }
    Ok(joins)
}

fn shortest_path(
    model: &SemanticModel,
    from_any: &[String],
    target: &str,
) -> Result<Vec<JoinStep>> {
    use std::collections::{HashMap, VecDeque};
    let mut graph: HashMap<String, Vec<JoinStep>> = HashMap::new();
    for rel in &model.relationships {
        graph.entry(rel.from_column.table.clone()).or_default().push(JoinStep {
            table: rel.to_column.table.clone(),
            left: rel.from_column.clone(),
            right: rel.to_column.clone(),
        });
        graph.entry(rel.to_column.table.clone()).or_default().push(JoinStep {
            table: rel.from_column.table.clone(),
            left: rel.to_column.clone(),
            right: rel.from_column.clone(),
        });
    }

    let mut queue = VecDeque::new();
    let mut prev: HashMap<String, (String, JoinStep)> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for start in from_any {
        queue.push_back(start.clone());
        seen.insert(start.clone());
    }
    while let Some(node) = queue.pop_front() {
        if node == target {
            let mut path = Vec::new();
            let mut cur = target.to_string();
            while !from_any.contains(&cur) {
                let (parent, step) = prev.get(&cur).ok_or_else(|| {
                    AnalyticsError::QueryPlan(format!("broken join path to `{target}`"))
                })?;
                path.push(step.clone());
                cur = parent.clone();
            }
            path.reverse();
            return Ok(path);
        }
        if let Some(edges) = graph.get(&node) {
            for step in edges {
                if seen.insert(step.table.clone()) {
                    prev.insert(step.table.clone(), (node.clone(), step.clone()));
                    queue.push_back(step.table.clone());
                }
            }
        }
    }
    Err(AnalyticsError::QueryPlan(format!(
        "no join path from {from_any:?} to `{target}` using accepted relationships"
    )))
}
