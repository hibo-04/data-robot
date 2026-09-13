use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::types::{ColumnRef, DataType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableKind {
    Table,
    View,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub name: String,
    pub tables: Vec<TableSchema>,
}

impl DatabaseSchema {
    pub fn table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.name.clone()).collect()
    }

    pub fn column_count(&self) -> usize {
        self.tables.iter().map(|t| t.columns.len()).sum()
    }

    pub fn column(&self, table: &str, column: &str) -> Option<&ColumnSchema> {
        self.table(table)?.columns.iter().find(|c| c.name == column)
    }

    pub fn is_primary_key(&self, table: &str, column: &str) -> bool {
        self.table(table)
            .and_then(|t| t.primary_key.as_ref())
            .map(|pk| pk.columns.iter().any(|c| c == column))
            .unwrap_or(false)
    }

    pub fn is_unique_column(&self, table: &str, column: &str) -> bool {
        if self.is_primary_key(table, column) {
            return true;
        }
        self.table(table)
            .map(|t| {
                t.unique_constraints.iter().any(|u| u.columns.len() == 1 && u.columns[0] == column)
            })
            .unwrap_or(false)
    }

    pub fn is_foreign_key_column(&self, table: &str, column: &str) -> bool {
        self.table(table)
            .map(|t| {
                t.foreign_keys
                    .iter()
                    .any(|fk| fk.columns.iter().any(|c| c == column))
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    pub kind: TableKind,
    pub columns: Vec<ColumnSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<PrimaryKey>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKey>,
    #[serde(default)]
    pub unique_constraints: Vec<UniqueConstraint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_row_count: Option<u64>,
}

impl TableSchema {
    pub fn qualified_name(&self) -> String {
        match &self.schema_name {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }

    pub fn column(&self, name: &str) -> Option<&ColumnSchema> {
        self.columns.iter().find(|c| c.name == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryKey {
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

impl ForeignKey {
    pub fn pairs(&self, from_table: &str) -> Vec<(ColumnRef, ColumnRef)> {
        self.columns
            .iter()
            .zip(self.ref_columns.iter())
            .map(|(from, to)| {
                (
                    ColumnRef::new(from_table, from),
                    ColumnRef::new(&self.ref_table, to),
                )
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct FixtureSchemaFile {
    #[serde(default)]
    pub name: Option<String>,
    pub tables: IndexMap<String, FixtureTableFile>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureTableFile {
    pub columns: IndexMap<String, FixtureColumnFile>,
    #[serde(default)]
    pub primary_key: Vec<String>,
    #[serde(default)]
    pub foreign_keys: Vec<FixtureForeignKeyFile>,
    #[serde(default)]
    pub unique: Vec<Vec<String>>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureColumnFile {
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(default)]
    pub nullable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureForeignKeyFile {
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

impl FixtureSchemaFile {
    pub fn into_schema(self, fallback_name: &str) -> DatabaseSchema {
        let tables = self
            .tables
            .into_iter()
            .map(|(name, table)| {
                let pk_set: Vec<String> = table.primary_key.clone();
                let columns = table
                    .columns
                    .into_iter()
                    .map(|(col_name, col)| {
                        let nullable = col.nullable.unwrap_or(!pk_set.iter().any(|c| c == &col_name));
                        ColumnSchema {
                            name: col_name,
                            data_type: DataType::parse(&col.data_type),
                            nullable,
                        }
                    })
                    .collect();
                TableSchema {
                    name,
                    schema_name: None,
                    kind: match table.kind.as_deref() {
                        Some("view") => TableKind::View,
                        _ => TableKind::Table,
                    },
                    columns,
                    primary_key: if table.primary_key.is_empty() {
                        None
                    } else {
                        Some(PrimaryKey {
                            columns: table.primary_key,
                        })
                    },
                    foreign_keys: table
                        .foreign_keys
                        .into_iter()
                        .map(|fk| ForeignKey {
                            columns: fk.columns,
                            ref_table: fk.ref_table,
                            ref_columns: fk.ref_columns,
                        })
                        .collect(),
                    unique_constraints: table
                        .unique
                        .into_iter()
                        .map(|columns| UniqueConstraint { columns })
                        .collect(),
                    approximate_row_count: None,
                }
            })
            .collect();

        DatabaseSchema {
            name: self
                .name
                .unwrap_or_else(|| fallback_name.to_string()),
            tables,
        }
    }
}
