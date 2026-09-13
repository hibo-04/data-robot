use serde::{Deserialize, Serialize};

use crate::connector::{overlap_value_limit, DataSource, DistinctSet};
use crate::error::Result;
use crate::profiling::{ColumnProfile, ColumnRole, DatabaseProfile};
use crate::schema::DatabaseSchema;
use crate::types::ColumnRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipSource {
    DatabaseConstraint,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipBand {
    Declared,
    Inferred,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    pub datatype_match: f64,
    pub name_similarity: f64,
    pub target_unique: bool,
    pub target_is_primary_key: bool,
    pub value_coverage: f64,
    pub reverse_coverage: f64,
    pub from_null_ratio: f64,
    pub truncated_values: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: ColumnRef,
    pub to: ColumnRef,
    pub relationship_type: RelationshipType,
    pub confidence: f64,
    pub source: RelationshipSource,
    pub band: RelationshipBand,
    pub evidence: RelationshipEvidence,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSet {
    pub relationships: Vec<Relationship>,
}

impl RelationshipSet {
    pub fn accepted(&self) -> impl Iterator<Item = &Relationship> {
        self.relationships
            .iter()
            .filter(|r| matches!(r.band, RelationshipBand::Declared | RelationshipBand::Inferred))
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let mut declared = 0;
        let mut inferred = 0;
        let mut uncertain = 0;
        for r in &self.relationships {
            match r.band {
                RelationshipBand::Declared => declared += 1,
                RelationshipBand::Inferred => inferred += 1,
                RelationshipBand::Uncertain => uncertain += 1,
            }
        }
        (declared, inferred, uncertain)
    }
}

const INFERRED_MIN: f64 = 0.80;
const UNCERTAIN_MIN: f64 = 0.55;

pub fn detect_relationships<S: DataSource>(
    source: &S,
    schema: &DatabaseSchema,
    profiles: &DatabaseProfile,
) -> Result<RelationshipSet> {
    let mut relationships = Vec::new();

    for table in &schema.tables {
        for fk in &table.foreign_keys {
            for (from, to) in fk.pairs(&table.name) {
                let evidence = RelationshipEvidence {
                    datatype_match: 1.0,
                    name_similarity: name_similarity(&from.table, &from.column, &to.table, &to.column),
                    target_unique: schema.is_unique_column(&to.table, &to.column),
                    target_is_primary_key: schema.is_primary_key(&to.table, &to.column),
                    value_coverage: 1.0,
                    reverse_coverage: 0.0,
                    from_null_ratio: profiles
                        .column(&from.table, &from.column)
                        .map(|c| c.null_ratio)
                        .unwrap_or(0.0),
                    truncated_values: false,
                    notes: vec!["declared as a database foreign key constraint".to_string()],
                };
                relationships.push(Relationship {
                    from,
                    to,
                    relationship_type: RelationshipType::ManyToOne,
                    confidence: 1.0,
                    source: RelationshipSource::DatabaseConstraint,
                    band: RelationshipBand::Declared,
                    evidence,
                    reason: format!(
                        "declared foreign key {}.{} → {}.{}",
                        table.name,
                        fk.columns.join(","),
                        fk.ref_table,
                        fk.ref_columns.join(",")
                    ),
                });
            }
        }
    }

    let declared_pairs: Vec<(String, String)> = relationships
        .iter()
        .map(|r| (r.from.qualified(), r.to.qualified()))
        .collect();

    let from_cols = candidate_from_columns(schema, profiles);
    let to_cols = candidate_to_columns(schema, profiles);

    for from in &from_cols {
        for to in &to_cols {
            if from.table == to.table && from.column == to.column {
                continue;
            }
            if from.table == to.table {
                continue;
            }
            if declared_pairs
                .iter()
                .any(|(a, b)| a == &from.qualified() && b == &to.qualified())
            {
                continue;
            }
            let Some(from_ty) = schema.column(&from.table, &from.column).map(|c| c.data_type.clone()) else {
                continue;
            };
            let Some(to_ty) = schema.column(&to.table, &to.column).map(|c| c.data_type.clone()) else {
                continue;
            };
            let datatype_match = from_ty.compatibility(&to_ty);
            if datatype_match < 0.4 {
                continue;
            }
            let name_sim = name_similarity(&from.table, &from.column, &to.table, &to.column);
            // Integer id spaces overlap by chance (1..N). Name evidence is required.
            if name_sim < 0.62 {
                continue;
            }

            let from_set = source.distinct_values(&from.table, &from.column, overlap_value_limit())?;
            let to_set = source.distinct_values(&to.table, &to.column, overlap_value_limit())?;
            let value_coverage = from_set.coverage_of(&to_set);
            let reverse_coverage = to_set.coverage_of(&from_set);
            let from_profile = profiles.column(&from.table, &from.column);
            let to_profile = profiles.column(&to.table, &to.column);

            let evidence = RelationshipEvidence {
                datatype_match: round4(datatype_match),
                name_similarity: round4(name_sim),
                target_unique: schema.is_unique_column(&to.table, &to.column)
                    || to_profile.map(|p| p.distinct_ratio >= 0.98).unwrap_or(false),
                target_is_primary_key: schema.is_primary_key(&to.table, &to.column),
                value_coverage: round4(value_coverage),
                reverse_coverage: round4(reverse_coverage),
                from_null_ratio: from_profile.map(|p| p.null_ratio).unwrap_or(0.0),
                truncated_values: from_set.truncated || to_set.truncated,
                notes: overlap_notes(&from_set, &to_set, value_coverage, name_sim),
            };

            let (confidence, rel_type) = score_relationship(&evidence, from_profile, to_profile);
            if confidence < UNCERTAIN_MIN {
                continue;
            }
            let band = if confidence >= INFERRED_MIN {
                RelationshipBand::Inferred
            } else {
                RelationshipBand::Uncertain
            };
            relationships.push(Relationship {
                from: from.clone(),
                to: to.clone(),
                relationship_type: rel_type,
                confidence: round4(confidence),
                source: RelationshipSource::Inferred,
                band,
                reason: explain(&evidence, confidence, rel_type),
                evidence,
            });
        }
    }

    keep_best_inferred_per_source(&mut relationships);

    relationships.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.from.qualified().cmp(&b.from.qualified()))
    });
    Ok(RelationshipSet { relationships })
}

fn candidate_from_columns(schema: &DatabaseSchema, profiles: &DatabaseProfile) -> Vec<ColumnRef> {
    let mut out = Vec::new();
    for table in &schema.tables {
        for col in &table.columns {
            if schema.is_primary_key(&table.name, &col.name) {
                continue;
            }
            if table
                .foreign_keys
                .iter()
                .any(|fk| fk.columns.iter().any(|c| c == &col.name))
            {
                continue;
            }
            let role = profiles
                .column(&table.name, &col.name)
                .map(|c| c.role)
                .unwrap_or(ColumnRole::Unknown);
            let name = col.name.to_ascii_lowercase();
            let looks_key = crate::profiling::is_key_name(&name);
            if matches!(
                role,
                ColumnRole::ForeignKeyCandidate | ColumnRole::Identifier
            ) || looks_key
            {
                out.push(ColumnRef::new(&table.name, &col.name));
            }
        }
    }
    out
}

fn keep_best_inferred_per_source(relationships: &mut Vec<Relationship>) {
    let mut best: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (idx, rel) in relationships.iter().enumerate() {
        if rel.source != RelationshipSource::Inferred {
            continue;
        }
        let key = rel.from.qualified();
        match best.get(&key) {
            None => {
                best.insert(key, idx);
            }
            Some(&prev) => {
                let other = &relationships[prev];
                let better = rel.evidence.name_similarity > other.evidence.name_similarity
                    || (rel.evidence.name_similarity == other.evidence.name_similarity
                        && rel.confidence > other.confidence);
                if better {
                    best.insert(key, idx);
                }
            }
        }
    }
    let keep: std::collections::HashSet<usize> = relationships
        .iter()
        .enumerate()
        .filter_map(|(idx, rel)| {
            if rel.source == RelationshipSource::DatabaseConstraint {
                Some(idx)
            } else {
                best.get(&rel.from.qualified()).copied().filter(|b| *b == idx)
            }
        })
        .collect();
    let mut i = 0;
    relationships.retain(|_| {
        let keep_row = keep.contains(&i);
        i += 1;
        keep_row
    });
}

fn candidate_to_columns(schema: &DatabaseSchema, profiles: &DatabaseProfile) -> Vec<ColumnRef> {
    let mut out = Vec::new();
    for table in &schema.tables {
        for col in &table.columns {
            let unique = schema.is_unique_column(&table.name, &col.name);
            let pk = schema.is_primary_key(&table.name, &col.name);
            let distinct_hi = profiles
                .column(&table.name, &col.name)
                .map(|c| c.distinct_ratio >= 0.95)
                .unwrap_or(false);
            let name = col.name.to_ascii_lowercase();
            let generic = is_generic_id(&name) || crate::profiling::is_key_name(&name);
            if pk || unique || (generic && distinct_hi) {
                out.push(ColumnRef::new(&table.name, &col.name));
            }
        }
    }
    out
}

fn score_relationship(
    evidence: &RelationshipEvidence,
    from: Option<&ColumnProfile>,
    to: Option<&ColumnProfile>,
) -> (f64, RelationshipType) {
    let unique_score = if evidence.target_unique { 1.0 } else { 0.25 };
    let pk_score = if evidence.target_is_primary_key { 1.0 } else { 0.0 };
    let mut confidence = 0.20 * evidence.datatype_match
        + 0.25 * evidence.name_similarity
        + 0.15 * unique_score
        + 0.10 * pk_score
        + 0.30 * evidence.value_coverage;

    if evidence.value_coverage < 0.50 {
        confidence *= 0.35;
    } else if evidence.value_coverage < 0.80 {
        confidence *= 0.75;
    }
    if evidence.name_similarity < 0.40 && evidence.value_coverage < 0.97 {
        confidence *= 0.65;
    }
    if evidence.name_similarity < 0.75 {
        confidence *= 0.55;
    }
    if evidence.truncated_values {
        confidence *= 0.9;
    }
    if evidence.from_null_ratio > 0.5 {
        confidence *= 0.9;
    }

    let from_distinct = from.map(|c| c.distinct_ratio).unwrap_or(0.0);
    let rel_type = if evidence.target_unique && from_distinct >= 0.98 {
        RelationshipType::OneToOne
    } else if evidence.target_unique {
        RelationshipType::ManyToOne
    } else if from_distinct >= 0.98 {
        RelationshipType::OneToMany
    } else {
        RelationshipType::ManyToMany
    };

    let _ = to;
    (confidence.clamp(0.0, 1.0), rel_type)
}

fn explain(evidence: &RelationshipEvidence, confidence: f64, rel_type: RelationshipType) -> String {
    format!(
        "score {:.2} = 0.20*datatype({:.2}) + 0.25*name({:.2}) + 0.15*unique({}) + 0.10*pk({}) + 0.30*coverage({:.2}); type {:?}",
        confidence,
        evidence.datatype_match,
        evidence.name_similarity,
        evidence.target_unique,
        evidence.target_is_primary_key,
        evidence.value_coverage,
        rel_type
    )
}

fn overlap_notes(
    from: &DistinctSet,
    to: &DistinctSet,
    coverage: f64,
    name_sim: f64,
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.push(format!(
        "value coverage {:.3} over {} non-null source values",
        coverage, from.non_null_count
    ));
    notes.push(format!(
        "target distinct values considered: {}",
        to.counts.len()
    ));
    notes.push(format!("name similarity {:.3}", name_sim));
    if from.truncated || to.truncated {
        notes.push("distinct-value collection was truncated; coverage is approximate".to_string());
    }
    notes
}

pub fn name_similarity(from_table: &str, from_col: &str, to_table: &str, to_col: &str) -> f64 {
    let from_col_n = normalize(from_col);
    let to_col_n = normalize(to_col);
    let from_table_n = normalize(from_table);
    let to_table_n = normalize(to_table);

    if is_generic_id(&from_col_n) && is_generic_id(&to_col_n) {
        return 0.15 * jaro(&from_table_n, &to_table_n);
    }

    let col_sim = if is_generic_id(&to_col_n) {
        0.0
    } else {
        jaro(&from_col_n, &to_col_n)
    };
    let from_stem = singularize(&strip_key_suffix(&from_col_n));
    let to_table_stem = singularize(&to_table_n);
    let to_col_stem = singularize(&strip_key_suffix(&to_col_n));
    let stem_to_table = jaro(&from_stem, &to_table_stem);
    let stem_to_col = jaro(&from_stem, &to_col_stem);
    let table_to_table = jaro(&singularize(&from_table_n), &to_table_stem);

    col_sim
        .max(stem_to_table)
        .max(stem_to_col)
        .max(if stem_to_table > 0.85 { 0.92 } else { 0.0 })
        .max(0.25 * table_to_table)
}

fn normalize(s: &str) -> String {
    s.to_ascii_lowercase().replace(['-', ' '], "_")
}

fn is_generic_id(name: &str) -> bool {
    matches!(name, "id" | "pk" | "uuid" | "guid" | "key")
}

fn strip_key_suffix(name: &str) -> String {
    for suf in ["_ids", "_id", "_nr", "_no", "_num", "_number", "_key", "_fk", "_pk", "_code", "_sku", "_uuid"] {
        if let Some(stripped) = name.strip_suffix(suf) {
            if !stripped.is_empty() {
                return stripped.to_string();
            }
        }
    }
    name.to_string()
}

fn singularize(s: &str) -> String {
    if s.ends_with("ies") && s.len() > 3 {
        return format!("{}y", &s[..s.len() - 3]);
    }
    if s.ends_with("sses") {
        return s.to_string();
    }
    if s.ends_with('s') && !s.ends_with("ss") && s.len() > 1 {
        return s[..s.len() - 1].to_string();
    }
    s.to_string()
}

fn jaro(a: &str, b: &str) -> f64 {
    if a == b {
        1.0
    } else {
        strsim::jaro_winkler(a, b)
    }
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn customer_nr_matches_customers() {
        let sim = name_similarity("orders", "customer_nr", "customers", "id");
        assert!(sim > 0.85, "got {sim}");
    }

    #[test]
    fn generic_ids_do_not_match_across_tables() {
        let sim = name_similarity("orders", "id", "customers", "id");
        assert!(sim < 0.3, "got {sim}");
    }

    #[test]
    fn product_sku_matches_products() {
        let sim = name_similarity("order_items", "product_sku", "products", "sku");
        assert!(sim > 0.85, "got {sim}");
    }
}
