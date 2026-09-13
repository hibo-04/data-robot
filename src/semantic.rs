use serde::{Deserialize, Serialize};

use crate::profiling::{ColumnProfile, ColumnRole, DatabaseProfile};
use crate::relationships::{RelationshipSet, RelationshipType};
use crate::schema::DatabaseSchema;
use crate::types::ColumnRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticModel {
    pub source_name: String,
    pub entities: Vec<Entity>,
    pub relationships: Vec<SemanticRelationship>,
    pub columns: Vec<SemanticColumn>,
}

impl SemanticModel {
    pub fn entity(&self, table: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.table == table)
    }

    pub fn column(&self, table: &str, column: &str) -> Option<&SemanticColumn> {
        self.columns
            .iter()
            .find(|c| c.ref_.table == table && c.ref_.column == column)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SemanticColumn> {
        let needle = name.to_ascii_lowercase();
        self.columns.iter().find(|c| {
            c.ref_.column.eq_ignore_ascii_case(&needle)
                || c.ref_.qualified().eq_ignore_ascii_case(&needle)
                || c.semantic_name.eq_ignore_ascii_case(&needle)
        })
    }

    pub fn time_dimensions(&self) -> Vec<&SemanticColumn> {
        self.columns
            .iter()
            .filter(|c| matches!(c.role, SemanticRole::TimeDimension))
            .collect()
    }

    pub fn measures(&self) -> Vec<&SemanticColumn> {
        self.columns
            .iter()
            .filter(|c| matches!(c.role, SemanticRole::Measure))
            .collect()
    }

    pub fn dimensions(&self) -> Vec<&SemanticColumn> {
        self.columns
            .iter()
            .filter(|c| {
                matches!(
                    c.role,
                    SemanticRole::Dimension | SemanticRole::CategoricalDimension
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub table: String,
    pub grain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelationship {
    pub from_entity: String,
    pub to_entity: String,
    pub from_column: ColumnRef,
    pub to_column: ColumnRef,
    pub relationship_type: RelationshipType,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    Identifier,
    ForeignKey,
    Measure,
    Dimension,
    CategoricalDimension,
    TimeDimension,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticColumn {
    #[serde(rename = "ref")]
    pub ref_: ColumnRef,
    pub semantic_name: String,
    pub role: SemanticRole,
    pub reason: String,
}

pub fn build_semantic_model(
    source_name: &str,
    schema: &DatabaseSchema,
    profiles: &DatabaseProfile,
    relationships: &RelationshipSet,
) -> SemanticModel {
    let entities = schema
        .tables
        .iter()
        .map(|t| Entity {
            name: entity_name(&t.name),
            table: t.name.clone(),
            grain: t.primary_key.as_ref().map(|pk| pk.columns.join(",")),
        })
        .collect();

    let semantic_rels = relationships
        .accepted()
        .map(|r| SemanticRelationship {
            from_entity: entity_name(&r.from.table),
            to_entity: entity_name(&r.to.table),
            from_column: r.from.clone(),
            to_column: r.to.clone(),
            relationship_type: r.relationship_type,
            confidence: r.confidence,
        })
        .collect();

    let fk_cols: Vec<String> = relationships
        .accepted()
        .map(|r| r.from.qualified())
        .collect();

    let mut columns = Vec::new();
    for table in &schema.tables {
        for col in &table.columns {
            let profile = profiles.column(&table.name, &col.name);
            let qualified = format!("{}.{}", table.name, col.name);
            let (role, reason) = if fk_cols.iter().any(|c| c == &qualified) {
                (
                    SemanticRole::ForeignKey,
                    "used as the many-side of an accepted relationship".to_string(),
                )
            } else {
                map_role(profile)
            };
            columns.push(SemanticColumn {
                ref_: ColumnRef::new(&table.name, &col.name),
                semantic_name: semantic_name(&table.name, &col.name),
                role,
                reason,
            });
        }
    }

    SemanticModel {
        source_name: source_name.to_string(),
        entities,
        relationships: semantic_rels,
        columns,
    }
}

fn map_role(profile: Option<&ColumnProfile>) -> (SemanticRole, String) {
    let Some(p) = profile else {
        return (SemanticRole::Unknown, "no profile available".to_string());
    };
    let role = match p.role {
        ColumnRole::Identifier => SemanticRole::Identifier,
        ColumnRole::ForeignKeyCandidate => SemanticRole::ForeignKey,
        ColumnRole::MeasureCandidate => SemanticRole::Measure,
        ColumnRole::DimensionCandidate => SemanticRole::Dimension,
        ColumnRole::TimeDimensionCandidate => SemanticRole::TimeDimension,
        ColumnRole::CategoricalDimension => SemanticRole::CategoricalDimension,
        ColumnRole::Unknown => SemanticRole::Unknown,
    };
    (role, p.role_reason.clone())
}

pub fn entity_name(table: &str) -> String {
    let mut name = table.to_string();
    if name.ends_with("ies") && name.len() > 3 {
        name = format!("{}y", &name[..name.len() - 3]);
    } else if name.ends_with('s') && !name.ends_with("ss") && name.len() > 1 {
        name.truncate(name.len() - 1);
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => {
            let mut s = c.to_uppercase().collect::<String>();
            s.push_str(chars.as_str());
            s
        }
        None => name,
    }
}

fn semantic_name(table: &str, column: &str) -> String {
    format!("{}.{}", entity_name(table), column)
}
