use std::collections::BTreeMap;

use chrono::NaiveDate;
use tokio::runtime::Handle;
use tokio_postgres::{Client, NoTls};

use crate::connector::{
    numeric_sample_from_query, DistinctSet, NumericColumnSpec, NumericSample, RawColumnProfile,
    RawDatabaseProfile, RawTableProfile, ValueCount,
};
use crate::error::{AnalyticsError, Result};
use crate::limits::TOP_VALUES;
use crate::query::{QueryResult, sql::quote_ident};
use crate::schema::{
    ColumnSchema, DatabaseSchema, ForeignKey, PrimaryKey, TableKind, TableSchema, UniqueConstraint,
};
use crate::types::{DataType, SqlDialect};

use super::DataSource;

/// First real database adapter. Mapping from catalogs into the generic schema
/// lives here; relationship/KPI/report code must not import this module.
pub struct PostgresConnector {
    name: String,
    client: Client,
}

impl PostgresConnector {
    pub async fn connect(url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(url, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Self {
            name: "postgres".to_string(),
            client,
        })
    }

    fn block_on<T>(&self, fut: impl std::future::Future<Output = Result<T>>) -> Result<T> {
        match Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new().map_err(|e| AnalyticsError::msg(e.to_string()))?;
                rt.block_on(fut)
            }
        }
    }
}

impl DataSource for PostgresConnector {
    fn name(&self) -> &str {
        &self.name
    }

    fn dialect(&self) -> SqlDialect {
        SqlDialect::Postgres
    }

    fn schema(&self) -> Result<DatabaseSchema> {
        self.block_on(load_schema(&self.client))
    }

    fn profile_raw(&self) -> Result<RawDatabaseProfile> {
        let schema = self.schema()?;
        self.block_on(profile_schema(&self.client, &schema))
    }

    fn distinct_values(
        &self,
        table: &str,
        column: &str,
        max_values: usize,
    ) -> Result<DistinctSet> {
        let schema = self.schema()?;
        let tbl = schema
            .table(table)
            .ok_or_else(|| AnalyticsError::UnknownTable(table.to_string()))?;
        if tbl.column(column).is_none() {
            return Err(AnalyticsError::UnknownColumn {
                table: table.to_string(),
                column: column.to_string(),
            });
        }
        self.block_on(distinct_values(&self.client, tbl, column, max_values))
    }

    fn execute_sql(&self, sql: &str) -> Result<QueryResult> {
        self.block_on(execute_sql(&self.client, sql))
    }

    fn sample_numeric(&self, table: &str, columns: &[String], max_rows: usize) -> Result<NumericSample> {
        if columns.is_empty() || max_rows == 0 {
            return Ok(NumericSample {
                label: table.to_string(),
                columns: Vec::new(),
            });
        }
        let schema = self.schema()?;
        let tbl = schema
            .table(table)
            .ok_or_else(|| AnalyticsError::UnknownTable(table.to_string()))?;
        let qtable = qualified_table(tbl)?;
        let mut selects = Vec::new();
        let mut specs = Vec::new();
        for (i, col) in columns.iter().enumerate() {
            if tbl.column(col).is_none() {
                return Err(AnalyticsError::UnknownColumn {
                    table: table.to_string(),
                    column: col.clone(),
                });
            }
            let alias = format!("c{i}");
            selects.push(format!("{}::float8 AS {}", quote_ident(col)?, quote_ident(&alias)?));
            specs.push(NumericColumnSpec {
                table: table.to_string(),
                column: col.clone(),
                summed: false,
            });
        }
        let sql = format!(
            "SELECT {} FROM {} LIMIT {}",
            selects.join(", "),
            qtable,
            max_rows
        );
        let result = self.execute_sql(&sql)?;
        Ok(numeric_sample_from_query(table, specs, &result.rows))
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
        let schema = self.schema()?;
        let left = schema
            .table(left_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(left_table.to_string()))?;
        let right = schema
            .table(right_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(right_table.to_string()))?;
        require_column(left, left_table, left_key)?;
        require_column(right, right_table, right_key)?;
        let mut selects = Vec::new();
        let mut specs = Vec::new();
        for (i, col) in left_columns.iter().enumerate() {
            require_column(left, left_table, col)?;
            let alias = format!("l{i}");
            selects.push(format!(
                "l.{}::float8 AS {}",
                quote_ident(col)?,
                quote_ident(&alias)?
            ));
            specs.push(NumericColumnSpec {
                table: left_table.to_string(),
                column: col.clone(),
                summed: false,
            });
        }
        for (i, col) in right_columns.iter().enumerate() {
            require_column(right, right_table, col)?;
            let alias = format!("r{i}");
            selects.push(format!(
                "r.{}::float8 AS {}",
                quote_ident(col)?,
                quote_ident(&alias)?
            ));
            specs.push(NumericColumnSpec {
                table: right_table.to_string(),
                column: col.clone(),
                summed: false,
            });
        }
        if selects.is_empty() {
            return Ok(NumericSample {
                label: format!("{left_table}⋈{right_table}"),
                columns: Vec::new(),
            });
        }
        let sql = format!(
            "SELECT {} FROM {} l JOIN {} r ON l.{} = r.{} LIMIT {}",
            selects.join(", "),
            qualified_table(left)?,
            qualified_table(right)?,
            quote_ident(left_key)?,
            quote_ident(right_key)?,
            max_rows
        );
        let result = self.execute_sql(&sql)?;
        Ok(numeric_sample_from_query(
            format!("{left_table}⋈{right_table}"),
            specs,
            &result.rows,
        ))
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
        let schema = self.schema()?;
        let child = schema
            .table(child_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(child_table.to_string()))?;
        let parent = schema
            .table(parent_table)
            .ok_or_else(|| AnalyticsError::UnknownTable(parent_table.to_string()))?;
        require_column(child, child_table, child_key)?;
        require_column(child, child_table, child_measure)?;
        require_column(parent, parent_table, parent_key)?;
        let mut selects = vec!["s.sm::float8 AS child_sum".to_string()];
        let mut specs = vec![NumericColumnSpec {
            table: child_table.to_string(),
            column: child_measure.to_string(),
            summed: true,
        }];
        for (i, col) in parent_measures.iter().enumerate() {
            require_column(parent, parent_table, col)?;
            let alias = format!("p{i}");
            selects.push(format!(
                "p.{}::float8 AS {}",
                quote_ident(col)?,
                quote_ident(&alias)?
            ));
            specs.push(NumericColumnSpec {
                table: parent_table.to_string(),
                column: col.clone(),
                summed: false,
            });
        }
        let sql = format!(
            "SELECT {selects} FROM {parent} p JOIN (
                SELECT {ck} AS k, SUM({cm}::float8) AS sm
                FROM {child}
                GROUP BY {ck}
             ) s ON s.k = p.{pk}
             LIMIT {max_groups}",
            selects = selects.join(", "),
            parent = qualified_table(parent)?,
            child = qualified_table(child)?,
            ck = quote_ident(child_key)?,
            cm = quote_ident(child_measure)?,
            pk = quote_ident(parent_key)?,
        );
        let result = self.execute_sql(&sql)?;
        Ok(numeric_sample_from_query(
            format!("sum({child_table}.{child_measure})@{parent_table}"),
            specs,
            &result.rows,
        ))
    }
}

fn require_column(table: &TableSchema, table_name: &str, column: &str) -> Result<()> {
    if table.column(column).is_none() {
        return Err(AnalyticsError::UnknownColumn {
            table: table_name.to_string(),
            column: column.to_string(),
        });
    }
    Ok(())
}

async fn load_schema(client: &Client) -> Result<DatabaseSchema> {
    let tables = client
        .query(
            r#"
            SELECT n.nspname AS schema_name,
                   c.relname AS table_name,
                   CASE c.relkind WHEN 'v' THEN 'view' ELSE 'table' END AS kind,
                   COALESCE(c.reltuples, -1)::bigint AS est_rows
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relkind IN ('r', 'p', 'v')
              AND n.nspname NOT IN ('pg_catalog', 'information_schema')
              AND n.nspname NOT LIKE 'pg_%'
            ORDER BY n.nspname, c.relname
            "#,
            &[],
        )
        .await?;

    let columns = client
        .query(
            r#"
            SELECT table_schema, table_name, column_name, udt_name, is_nullable
            FROM information_schema.columns
            WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
              AND table_schema NOT LIKE 'pg_%'
            ORDER BY table_schema, table_name, ordinal_position
            "#,
            &[],
        )
        .await?;

    let pks = client
        .query(
            r#"
            SELECT kcu.table_schema, kcu.table_name, kcu.column_name, kcu.ordinal_position
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
              ON tc.constraint_name = kcu.constraint_name
             AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
              AND tc.table_schema NOT IN ('pg_catalog', 'information_schema')
            ORDER BY kcu.table_schema, kcu.table_name, kcu.ordinal_position
            "#,
            &[],
        )
        .await?;

    let uniques = client
        .query(
            r#"
            SELECT kcu.table_schema, kcu.table_name, kcu.column_name, tc.constraint_name, kcu.ordinal_position
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
              ON tc.constraint_name = kcu.constraint_name
             AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'UNIQUE'
              AND tc.table_schema NOT IN ('pg_catalog', 'information_schema')
            ORDER BY kcu.table_schema, kcu.table_name, tc.constraint_name, kcu.ordinal_position
            "#,
            &[],
        )
        .await?;

    let fks = client
        .query(
            r#"
            SELECT
                kcu.table_schema,
                kcu.table_name,
                kcu.column_name,
                ccu.table_name AS ref_table,
                ccu.column_name AS ref_column,
                tc.constraint_name,
                kcu.ordinal_position
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
              ON tc.constraint_name = kcu.constraint_name
             AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage ccu
              ON ccu.constraint_name = tc.constraint_name
             AND ccu.table_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
              AND tc.table_schema NOT IN ('pg_catalog', 'information_schema')
            ORDER BY kcu.table_schema, kcu.table_name, tc.constraint_name, kcu.ordinal_position
            "#,
            &[],
        )
        .await?;

    let mut by_table: BTreeMap<(String, String), TableSchema> = BTreeMap::new();
    for row in &tables {
        let schema_name: String = row.get("schema_name");
        let table_name: String = row.get("table_name");
        let kind: String = row.get("kind");
        let est_rows: i64 = row.get("est_rows");
        by_table.insert(
            (schema_name.clone(), table_name.clone()),
            TableSchema {
                name: table_name,
                schema_name: Some(schema_name),
                kind: if kind == "view" {
                    TableKind::View
                } else {
                    TableKind::Table
                },
                columns: Vec::new(),
                primary_key: None,
                foreign_keys: Vec::new(),
                unique_constraints: Vec::new(),
                approximate_row_count: if est_rows >= 0 {
                    Some(est_rows as u64)
                } else {
                    None
                },
            },
        );
    }

    for row in &columns {
        let schema_name: String = row.get("table_schema");
        let table_name: String = row.get("table_name");
        let column_name: String = row.get("column_name");
        let udt: String = row.get("udt_name");
        let is_nullable: String = row.get("is_nullable");
        if let Some(table) = by_table.get_mut(&(schema_name, table_name)) {
            table.columns.push(ColumnSchema {
                name: column_name,
                data_type: map_postgres_type(&udt),
                nullable: is_nullable.eq_ignore_ascii_case("YES"),
            });
        }
    }

    let mut pk_map: BTreeMap<(String, String), Vec<(i32, String)>> = BTreeMap::new();
    for row in &pks {
        let key = (row.get::<_, String>("table_schema"), row.get::<_, String>("table_name"));
        pk_map
            .entry(key)
            .or_default()
            .push((row.get("ordinal_position"), row.get("column_name")));
    }
    for (key, mut cols) in pk_map {
        cols.sort_by_key(|(pos, _)| *pos);
        if let Some(table) = by_table.get_mut(&key) {
            table.primary_key = Some(PrimaryKey {
                columns: cols.into_iter().map(|(_, n)| n).collect(),
            });
        }
    }

    let mut uq_map: BTreeMap<(String, String, String), Vec<(i32, String)>> = BTreeMap::new();
    for row in &uniques {
        let key = (
            row.get::<_, String>("table_schema"),
            row.get::<_, String>("table_name"),
            row.get::<_, String>("constraint_name"),
        );
        uq_map
            .entry(key)
            .or_default()
            .push((row.get("ordinal_position"), row.get("column_name")));
    }
    for ((schema, table, _), mut cols) in uq_map {
        cols.sort_by_key(|(pos, _)| *pos);
        if let Some(table) = by_table.get_mut(&(schema, table)) {
            table.unique_constraints.push(UniqueConstraint {
                columns: cols.into_iter().map(|(_, n)| n).collect(),
            });
        }
    }

    let mut fk_map: BTreeMap<(String, String, String), Vec<(i32, String, String, String)>> =
        BTreeMap::new();
    for row in &fks {
        let key = (
            row.get::<_, String>("table_schema"),
            row.get::<_, String>("table_name"),
            row.get::<_, String>("constraint_name"),
        );
        fk_map.entry(key).or_default().push((
            row.get("ordinal_position"),
            row.get("column_name"),
            row.get("ref_table"),
            row.get("ref_column"),
        ));
    }
    for ((schema, table, _), mut cols) in fk_map {
        cols.sort_by_key(|(pos, _, _, _)| *pos);
        let ref_table = cols.first().map(|c| c.2.clone()).unwrap_or_default();
        if let Some(table) = by_table.get_mut(&(schema, table)) {
            table.foreign_keys.push(ForeignKey {
                columns: cols.iter().map(|c| c.1.clone()).collect(),
                ref_table,
                ref_columns: cols.iter().map(|c| c.3.clone()).collect(),
            });
        }
    }

    Ok(DatabaseSchema {
        name: "postgres".to_string(),
        tables: by_table.into_values().collect(),
    })
}

pub fn map_postgres_type(udt_name: &str) -> DataType {
    DataType::parse(udt_name)
}

async fn profile_schema(client: &Client, schema: &DatabaseSchema) -> Result<RawDatabaseProfile> {
    let mut tables = Vec::new();
    for table in &schema.tables {
        tables.push(profile_table(client, table).await?);
    }
    Ok(RawDatabaseProfile { tables })
}

async fn profile_table(client: &Client, table: &TableSchema) -> Result<RawTableProfile> {
    let qtable = qualified_table(table)?;
    let row_count: i64 = client
        .query_one(&format!("SELECT COUNT(*)::bigint FROM {qtable}"), &[])
        .await?
        .get(0);
    let mut columns = Vec::new();
    for col in &table.columns {
        columns.push(profile_column(client, table, col, row_count as u64).await?);
    }
    Ok(RawTableProfile {
        table: table.name.clone(),
        row_count: row_count as u64,
        columns,
    })
}

async fn profile_column(
    client: &Client,
    table: &TableSchema,
    col: &ColumnSchema,
    row_count: u64,
) -> Result<RawColumnProfile> {
    let qtable = qualified_table(table)?;
    let qcol = quote_ident(&col.name)?;
    let base = client
        .query_one(
            &format!(
                "SELECT COUNT({qcol})::bigint AS non_null, COUNT(DISTINCT {qcol})::bigint AS distinct_count FROM {qtable}"
            ),
            &[],
        )
        .await?;
    let non_null: i64 = base.get("non_null");
    let distinct_count: i64 = base.get("distinct_count");
    let null_count = row_count.saturating_sub(non_null as u64);

    let mut profile = RawColumnProfile {
        column: col.name.clone(),
        row_count,
        null_count,
        distinct_count: distinct_count as u64,
        min: None,
        max: None,
        avg: None,
        stddev: None,
        median: None,
        avg_length: None,
        min_date: None,
        max_date: None,
        top_values: Vec::new(),
    };

    if col.data_type.is_numeric() {
        let stats = client
            .query_one(
                &format!(
                    "SELECT MIN({qcol}::float8) AS min_v, MAX({qcol}::float8) AS max_v,
                            AVG({qcol}::float8) AS avg_v, STDDEV_POP({qcol}::float8) AS std_v,
                            PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY {qcol}::float8) AS med_v
                     FROM {qtable} WHERE {qcol} IS NOT NULL"
                ),
                &[],
            )
            .await;
        if let Ok(row) = stats {
            let min_v: Option<f64> = row.get("min_v");
            let max_v: Option<f64> = row.get("max_v");
            profile.min = min_v.map(|v| format!("{v}"));
            profile.max = max_v.map(|v| format!("{v}"));
            profile.avg = row.get("avg_v");
            profile.stddev = row.get("std_v");
            profile.median = row.get("med_v");
        }
    } else if col.data_type.is_temporal() {
        let stats = client
            .query_one(
                &format!(
                    "SELECT MIN({qcol}::date) AS min_d, MAX({qcol}::date) AS max_d FROM {qtable} WHERE {qcol} IS NOT NULL"
                ),
                &[],
            )
            .await;
        if let Ok(row) = stats {
            let min_d: Option<NaiveDate> = row.get("min_d");
            let max_d: Option<NaiveDate> = row.get("max_d");
            profile.min_date = min_d.map(|d| d.to_string());
            profile.max_date = max_d.map(|d| d.to_string());
            profile.min = profile.min_date.clone();
            profile.max = profile.max_date.clone();
        }
    } else if col.data_type.is_textish() {
        let avg_len = client
            .query_opt(
                &format!(
                    "SELECT AVG(LENGTH({qcol}::text)) FROM {qtable} WHERE {qcol} IS NOT NULL"
                ),
                &[],
            )
            .await?;
        if let Some(row) = avg_len {
            profile.avg_length = row.get(0);
        }
        let bounds = client
            .query_opt(
                &format!(
                    "SELECT MIN({qcol}::text), MAX({qcol}::text) FROM {qtable} WHERE {qcol} IS NOT NULL"
                ),
                &[],
            )
            .await?;
        if let Some(row) = bounds {
            profile.min = row.get(0);
            profile.max = row.get(1);
        }
    }

    let top = client
        .query(
            &format!(
                "SELECT {qcol}::text AS v, COUNT(*)::bigint AS c
                 FROM {qtable}
                 WHERE {qcol} IS NOT NULL
                 GROUP BY 1
                 ORDER BY c DESC, v
                 LIMIT {TOP_VALUES}"
            ),
            &[],
        )
        .await;
    if let Ok(rows) = top {
        profile.top_values = rows
            .into_iter()
            .map(|row| ValueCount {
                value: row.get::<_, Option<String>>("v").unwrap_or_default(),
                count: row.get::<_, i64>("c") as u64,
            })
            .collect();
    }

    Ok(profile)
}

async fn distinct_values(
    client: &Client,
    table: &TableSchema,
    column: &str,
    max_values: usize,
) -> Result<DistinctSet> {
    let qtable = qualified_table(table)?;
    let qcol = quote_ident(column)?;
    let rows = client
        .query(
            &format!(
                "SELECT {qcol}::text AS v, COUNT(*)::bigint AS c
                 FROM {qtable}
                 WHERE {qcol} IS NOT NULL
                 GROUP BY 1
                 ORDER BY c DESC
                 LIMIT {max_values}"
            ),
            &[],
        )
        .await?;
    let total = client
        .query_one(
            &format!("SELECT COUNT({qcol})::bigint FROM {qtable}"),
            &[],
        )
        .await?;
    let non_null: i64 = total.get(0);
    let mut counts = indexmap::IndexMap::new();
    for row in &rows {
        let v: Option<String> = row.get("v");
        let c: i64 = row.get("c");
        if let Some(v) = v {
            counts.insert(v, c as u64);
        }
    }
    let truncated = counts.len() >= max_values && (non_null as u64) > counts.values().sum::<u64>();
    Ok(DistinctSet {
        counts,
        non_null_count: non_null as u64,
        truncated,
    })
}

async fn execute_sql(client: &Client, sql: &str) -> Result<QueryResult> {
    let rows = client.query(sql, &[]).await?;
    if rows.is_empty() {
        return Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
        });
    }
    let columns: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    let mut out = Vec::new();
    for row in &rows {
        let mut rec = Vec::new();
        for i in 0..columns.len() {
            rec.push(pg_cell_to_string(row, i));
        }
        out.push(rec);
    }
    Ok(QueryResult {
        columns,
        rows: out,
    })
}

fn pg_cell_to_string(row: &tokio_postgres::Row, idx: usize) -> String {
    if let Ok(v) = row.try_get::<_, Option<String>>(idx) {
        return v.unwrap_or_else(|| "NULL".to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<i64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<i32>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<f64>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<bool>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<NaiveDate>>(idx) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
    }
    "NULL".to_string()
}

fn qualified_table(table: &TableSchema) -> Result<String> {
    match &table.schema_name {
        Some(schema) => Ok(format!("{}.{}", quote_ident(schema)?, quote_ident(&table.name)?)),
        None => quote_ident(&table.name),
    }
}
