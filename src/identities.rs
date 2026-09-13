//! Name-free numeric identity detection.
//!
//! Finds stored columns that reconstruct from other columns via a small algebra
//! (`+`, `-`, `*`, `/`, constant rate `k`, scaled product `k·a·b`, grain `SUM`).
//! Column names are never consulted; evidence is match ratio and residual size.
//! When a fixture formula is not found, extend this template set — see
//! `docs/model-review.md`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::connector::{DataSource, NumericColumn, NumericSample};
use crate::error::Result;
use crate::kpi::Certainty;
use crate::limits::{IDENTITY_MIN_PAIRS, IDENTITY_SAMPLE_ROWS};
use crate::profiling::{ColumnRole, DatabaseProfile};
use crate::relationships::{RelationshipSet, RelationshipType};
use crate::schema::DatabaseSchema;
use crate::types::ColumnRef;

const MIN_MATCH: f64 = 0.82;
const MAX_COLUMNS_PER_TABLE: usize = 20;
const NEAR_ZERO: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityScope {
    IntraTable,
    Join,
    Grain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityTemplate {
    Equal,
    Rate,
    Sum,
    Sum3,
    Difference,
    Product,
    ScaledProduct,
    Quotient,
    ProductMinus,
    ProductPlus,
    GrainSum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnIdentity {
    pub id: String,
    pub scope: IdentityScope,
    pub target: ColumnRef,
    pub inputs: Vec<ColumnRef>,
    pub template: IdentityTemplate,
    pub expression: String,
    pub match_ratio: f64,
    pub mae: f64,
    pub pair_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coefficient: Option<f64>,
    pub confidence: f64,
    pub certainty: Certainty,
    pub reason: String,
    /// Magnitude of the target column; used to prefer reconstructing totals over remainders.
    #[serde(skip)]
    pub target_scale: f64,
}

#[derive(Clone, Default)]
struct TableNumerics {
    /// Columns drawn into the sample (measures + percent/rate factors).
    sample: Vec<String>,
    /// Only these may be reconstructed as `y` (flags stay inputs).
    targets: HashSet<String>,
}

pub fn detect_identities<S: DataSource>(
    source: &S,
    schema: &DatabaseSchema,
    profiles: &DatabaseProfile,
    relationships: &RelationshipSet,
) -> Result<Vec<ColumnIdentity>> {
    let mut hits = Vec::new();
    let formula_cols = formula_columns(schema, profiles, relationships);

    for (table, cols) in &formula_cols {
        if cols.sample.len() < 2 {
            continue;
        }
        let sample = source.sample_numeric(table, &cols.sample, IDENTITY_SAMPLE_ROWS)?;
        hits.extend(detect_in_sample(&sample, IdentityScope::IntraTable, Some(&cols.targets)));
    }

    for rel in relationships.accepted() {
        let left = formula_cols.get(&rel.from.table).cloned().unwrap_or_default();
        let right = formula_cols.get(&rel.to.table).cloned().unwrap_or_default();
        if left.sample.is_empty() && right.sample.is_empty() {
            continue;
        }
        let joined = source.sample_join_numeric(
            &rel.from.table,
            &left.sample,
            &rel.to.table,
            &right.sample,
            &rel.from.column,
            &rel.to.column,
            IDENTITY_SAMPLE_ROWS,
        )?;
        let mut join_targets = left.targets.clone();
        join_targets.extend(right.targets.iter().cloned());
        hits.extend(detect_in_sample(&joined, IdentityScope::Join, Some(&join_targets)));

        let many_to_one = matches!(
            rel.relationship_type,
            RelationshipType::ManyToOne | RelationshipType::OneToMany
        );
        if many_to_one && !left.sample.is_empty() && !right.sample.is_empty() {
            let (
                child_table,
                child_key,
                parent_table,
                parent_key,
                child_cols,
                parent_cols,
                child_targets,
                parent_targets,
            ) = if matches!(rel.relationship_type, RelationshipType::OneToMany) {
                (
                    rel.to.table.clone(),
                    rel.to.column.clone(),
                    rel.from.table.clone(),
                    rel.from.column.clone(),
                    right.sample.clone(),
                    left.sample.clone(),
                    right.targets.clone(),
                    left.targets.clone(),
                )
            } else {
                (
                    rel.from.table.clone(),
                    rel.from.column.clone(),
                    rel.to.table.clone(),
                    rel.to.column.clone(),
                    left.sample.clone(),
                    right.sample.clone(),
                    left.targets.clone(),
                    right.targets.clone(),
                )
            };
            for child_measure in child_cols.iter().filter(|c| child_targets.contains(*c)) {
                let grain = source.sample_grouped_sum(
                    &child_table,
                    &child_key,
                    child_measure,
                    &parent_table,
                    &parent_key,
                    &parent_cols,
                    IDENTITY_SAMPLE_ROWS,
                )?;
                hits.extend(detect_in_sample(
                    &grain,
                    IdentityScope::Grain,
                    Some(&parent_targets),
                ));
            }
        }
    }

    Ok(dedup_hits(hits))
}

fn is_vat_reciprocal(k: f64) -> bool {
    let x = k.abs();
    (x - 0.840_336).abs() < 0.01
        || (x - 1.0 / 1.19).abs() < 0.01
        || (x - 1.0 / 0.19).abs() < 0.05
}

fn prune_noisy_identities(hits: Vec<ColumnIdentity>) -> Vec<ColumnIdentity> {
    let sum_sets: Vec<Vec<String>> = hits
        .iter()
        .filter(|h| matches!(h.template, IdentityTemplate::Sum | IdentityTemplate::Sum3))
        .map(|sum| {
            let mut cols: Vec<String> = std::iter::once(sum.target.qualified())
                .chain(sum.inputs.iter().map(|c| c.qualified()))
                .collect();
            cols.sort();
            cols
        })
        .collect();
    hits.into_iter()
        .filter(|h| {
            if h.scope == IdentityScope::Join && h.template == IdentityTemplate::Equal {
                return false;
            }
            if h.scope == IdentityScope::Join
                && matches!(
                    h.template,
                    IdentityTemplate::ProductPlus | IdentityTemplate::Quotient
                )
            {
                return false;
            }
            if h.scope == IdentityScope::Grain && h.template == IdentityTemplate::Quotient {
                return false;
            }
            if h.scope == IdentityScope::Join
                && matches!(
                    h.template,
                    IdentityTemplate::Rate
                        | IdentityTemplate::Equal
                        | IdentityTemplate::ProductPlus
                        | IdentityTemplate::Sum
                        | IdentityTemplate::Sum3
                        | IdentityTemplate::Difference
                )
                && h.match_ratio < 0.96
            {
                return false;
            }
            if h.scope == IdentityScope::Grain
                && matches!(h.template, IdentityTemplate::ProductMinus | IdentityTemplate::ProductPlus)
                && h.match_ratio < 0.96
            {
                return false;
            }
            if let Some(k) = h.coefficient {
                if is_vat_reciprocal(k) {
                    return false;
                }
            }
            if h.template == IdentityTemplate::Rate {
                let cols: Vec<String> = std::iter::once(h.target.qualified())
                    .chain(h.inputs.iter().map(|c| c.qualified()))
                    .collect();
                for sum_cols in &sum_sets {
                    if cols.iter().all(|c| sum_cols.iter().any(|s| s == c)) && cols.len() < sum_cols.len()
                    {
                        if let Some(k) = h.coefficient {
                            let tax_like = k > 0.04 && k < 0.35;
                            if !tax_like {
                                return false;
                            }
                        }
                    }
                }
            }
            true
        })
        .collect()
}

fn formula_columns(
    schema: &DatabaseSchema,
    profiles: &DatabaseProfile,
    relationships: &RelationshipSet,
) -> HashMap<String, TableNumerics> {
    let mut key_cols = HashSet::new();
    for rel in relationships.accepted() {
        key_cols.insert(rel.from.qualified());
        key_cols.insert(rel.to.qualified());
    }

    let mut out = HashMap::new();
    for table in &schema.tables {
        let mut scored = Vec::new();
        let mut targets = HashSet::new();
        for col in &table.columns {
            if !col.data_type.is_numeric() {
                continue;
            }
            if schema.is_primary_key(&table.name, &col.name) {
                continue;
            }
            if schema.is_foreign_key_column(&table.name, &col.name) {
                continue;
            }
            let qualified = format!("{}.{}", table.name, col.name);
            if key_cols.contains(&qualified) {
                continue;
            }
            let Some(profile) = profiles.column(&table.name, &col.name) else {
                continue;
            };
            if matches!(
                profile.role,
                ColumnRole::Identifier | ColumnRole::ForeignKeyCandidate
            ) {
                continue;
            }
            if profile.distinct_count <= 1 {
                continue;
            }
            if profile.stddev.unwrap_or(0.0).abs() < NEAR_ZERO {
                continue;
            }
            let flag_like =
                profile.role == ColumnRole::CategoricalDimension && profile.distinct_ratio <= 0.08;
            let factor = factor_like(profile);
            if flag_like && !factor {
                continue;
            }
            if !flag_like {
                targets.insert(col.name.clone());
            }
            scored.push((profile.stddev.unwrap_or(0.0).abs(), col.name.clone()));
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(MAX_COLUMNS_PER_TABLE);
        let sample: Vec<String> = scored.into_iter().map(|(_, n)| n).collect();
        targets.retain(|n| sample.iter().any(|s| s == n));
        if !sample.is_empty() {
            out.insert(table.name.clone(), TableNumerics { sample, targets });
        }
    }
    out
}

fn factor_like(profile: &crate::profiling::ColumnProfile) -> bool {
    let min = profile.min.as_ref().and_then(|s| s.parse::<f64>().ok());
    let max = profile.max.as_ref().and_then(|s| s.parse::<f64>().ok());
    let avg = profile.avg;
    let (Some(min), Some(max), Some(avg)) = (min, max, avg) else {
        return false;
    };
    if profile.distinct_count < 3 {
        return false;
    }
    if min >= -0.001 && max <= 1.001 && avg > 0.0 && avg < 1.0 {
        return true;
    }
    if min >= 0.0 && max <= 100.0 && avg >= 5.0 && avg <= 99.0 {
        return true;
    }
    false
}

#[derive(Clone)]
struct RawHit {
    yi: usize,
    inputs: Vec<usize>,
    template: IdentityTemplate,
    k: Option<f64>,
    match_ratio: f64,
    mae: f64,
    pairs: u64,
}

fn detect_in_sample(
    sample: &NumericSample,
    scope: IdentityScope,
    targets: Option<&HashSet<String>>,
) -> Vec<ColumnIdentity> {
    let n = sample.columns.len();
    if n < 2 || sample.n_rows() < IDENTITY_MIN_PAIRS as usize {
        return Vec::new();
    }

    let mut raw = Vec::new();
    for yi in 0..n {
        if sample.columns[yi].summed {
            continue;
        }
        if let Some(allowed) = targets {
            if !allowed.contains(&sample.columns[yi].column) {
                continue;
            }
        }
        for xi in 0..n {
            if xi == yi {
                continue;
            }
            if let Some(hit) = try_rate(&sample.columns, yi, xi, scope) {
                raw.push(hit);
            }
        }
        for a in 0..n {
            if a == yi {
                continue;
            }
            for b in (a + 1)..n {
                if b == yi {
                    continue;
                }
                if let Some(hit) = try_sum(&sample.columns, yi, a, b) {
                    raw.push(hit);
                }
                if let Some(hit) = try_diff(&sample.columns, yi, a, b) {
                    raw.push(hit);
                }
                if let Some(hit) = try_diff(&sample.columns, yi, b, a) {
                    raw.push(hit);
                }
                if let Some(hit) = try_product(&sample.columns, yi, a, b) {
                    raw.push(hit);
                }
                if let Some(hit) = try_scaled_product(&sample.columns, yi, a, b) {
                    raw.push(hit);
                }
                if let Some(hit) = try_quotient(&sample.columns, yi, a, b) {
                    raw.push(hit);
                }
                if let Some(hit) = try_quotient(&sample.columns, yi, b, a) {
                    raw.push(hit);
                }
            }
        }
        if n >= 4 {
            for a in 0..n {
                if a == yi {
                    continue;
                }
                for b in (a + 1)..n {
                    if b == yi {
                        continue;
                    }
                    for c in 0..n {
                        if c == yi || c == a || c == b {
                            continue;
                        }
                        if let Some(hit) = try_product_minus(&sample.columns, yi, a, b, c) {
                            raw.push(hit);
                        }
                        if let Some(hit) = try_product_plus(&sample.columns, yi, a, b, c) {
                            raw.push(hit);
                        }
                    }
                    for c in (b + 1)..n {
                        if c == yi {
                            continue;
                        }
                        if let Some(hit) = try_sum3(&sample.columns, yi, a, b, c) {
                            raw.push(hit);
                        }
                    }
                }
            }
        }
    }

    raw.into_iter()
        .map(|hit| to_identity(sample, hit, scope))
        .collect()
}

fn try_rate(cols: &[NumericColumn], yi: usize, xi: usize, scope: IdentityScope) -> Option<RawHit> {
    let y = &cols[yi].values;
    let x = &cols[xi].values;
    let mut ratios = Vec::new();
    for i in 0..y.len() {
        let (Some(yv), Some(xv)) = (y[i], x[i]) else {
            continue;
        };
        if xv.abs() < NEAR_ZERO {
            continue;
        }
        ratios.push(yv / xv);
    }
    if (ratios.len() as u64) < IDENTITY_MIN_PAIRS {
        return None;
    }
    let median_k = median(&ratios)?;
    if !median_k.is_finite() || median_k.abs() < 0.01 || median_k.abs() > 1_000.0 {
        return None;
    }
    let (pairs, hits, mae) = score_pred(y, |i| x[i].map(|xv| median_k * xv));
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH {
        return None;
    }
    let snapped = snap_ratio(median_k);
    let (pairs, mae, match_ratio, k) = if (snapped - median_k).abs() <= NEAR_ZERO {
        (pairs, mae, match_ratio, median_k)
    } else {
        let (p2, h2, mae2) = score_pred(y, |i| x[i].map(|xv| snapped * xv));
        match ratio(h2, p2) {
            Some(m2) if m2 >= match_ratio - 0.01 => (p2, mae2, m2, snapped),
            _ => (pairs, mae, match_ratio, median_k),
        }
    };
    if !residual_plausible(mae, y) {
        return None;
    }
    let template = if (k - 1.0).abs() <= 0.005 {
        if scope == IdentityScope::Grain && cols[xi].summed {
            IdentityTemplate::GrainSum
        } else {
            IdentityTemplate::Equal
        }
    } else if scope == IdentityScope::Grain && cols[xi].summed {
        IdentityTemplate::GrainSum
    } else {
        IdentityTemplate::Rate
    };
    Some(RawHit {
        yi,
        inputs: vec![xi],
        template,
        k: if matches!(template, IdentityTemplate::Equal) {
            None
        } else {
            Some(k)
        },
        match_ratio,
        mae,
        pairs,
    })
}

fn try_sum(cols: &[NumericColumn], yi: usize, a: usize, b: usize) -> Option<RawHit> {
    if !significant_addend(&cols[yi].values, &cols[a].values)
        || !significant_addend(&cols[yi].values, &cols[b].values)
    {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i]) {
        (Some(av), Some(bv)) => Some(av + bv),
        _ => None,
    });
    finish_binary(yi, &cols[yi].values, vec![a, b], IdentityTemplate::Sum, pairs, hits, mae)
}

fn try_diff(cols: &[NumericColumn], yi: usize, a: usize, b: usize) -> Option<RawHit> {
    if !significant_addend(&cols[yi].values, &cols[b].values) {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i]) {
        (Some(av), Some(bv)) => Some(av - bv),
        _ => None,
    });
    finish_binary(yi, &cols[yi].values, vec![a, b], IdentityTemplate::Difference, pairs, hits, mae)
}

fn try_product(cols: &[NumericColumn], yi: usize, a: usize, b: usize) -> Option<RawHit> {
    if looks_like_ones(&cols[a].values) || looks_like_ones(&cols[b].values) {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i]) {
        (Some(av), Some(bv)) => Some(av * bv),
        _ => None,
    });
    finish_binary(yi, &cols[yi].values, vec![a, b], IdentityTemplate::Product, pairs, hits, mae)
}

fn try_scaled_product(cols: &[NumericColumn], yi: usize, a: usize, b: usize) -> Option<RawHit> {
    if looks_like_ones(&cols[a].values) || looks_like_ones(&cols[b].values) {
        return None;
    }
    let y = &cols[yi].values;
    let mut ratios = Vec::new();
    for i in 0..y.len() {
        let (Some(yv), Some(av), Some(bv)) = (y[i], cols[a].values[i], cols[b].values[i]) else {
            continue;
        };
        let prod = av * bv;
        if prod.abs() < NEAR_ZERO {
            continue;
        }
        ratios.push(yv / prod);
    }
    if (ratios.len() as u64) < IDENTITY_MIN_PAIRS {
        return None;
    }
    let median_k = median(&ratios)?;
    if !median_k.is_finite() || median_k.abs() < 1e-4 || median_k.abs() > 1_000_000.0 {
        return None;
    }
    if (median_k - 1.0).abs() <= 0.03 {
        return None;
    }
    let snapped = snap_ratio(median_k);
    let k = if (snapped - 1.0).abs() <= 0.03 {
        median_k
    } else {
        snapped
    };
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i]) {
        (Some(av), Some(bv)) => Some(k * av * bv),
        _ => None,
    });
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH || !residual_plausible(mae, y) {
        return None;
    }
    Some(RawHit {
        yi,
        inputs: vec![a, b],
        template: IdentityTemplate::ScaledProduct,
        k: Some(k),
        match_ratio,
        mae,
        pairs,
    })
}

fn try_quotient(cols: &[NumericColumn], yi: usize, a: usize, b: usize) -> Option<RawHit> {
    if looks_like_ones(&cols[b].values) {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i]) {
        (Some(av), Some(bv)) if bv.abs() > NEAR_ZERO => Some(av / bv),
        _ => None,
    });
    finish_binary(yi, &cols[yi].values, vec![a, b], IdentityTemplate::Quotient, pairs, hits, mae)
}

fn try_product_minus(
    cols: &[NumericColumn],
    yi: usize,
    a: usize,
    b: usize,
    c: usize,
) -> Option<RawHit> {
    if looks_like_ones(&cols[a].values) || looks_like_ones(&cols[b].values) {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i], cols[c].values[i]) {
        (Some(av), Some(bv), Some(cv)) => Some(av * bv - cv),
        _ => None,
    });
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH {
        return None;
    }
    if !residual_plausible(mae, y) {
        return None;
    }
    Some(RawHit {
        yi,
        inputs: vec![a, b, c],
        template: IdentityTemplate::ProductMinus,
        k: None,
        match_ratio,
        mae,
        pairs,
    })
}

fn try_product_plus(
    cols: &[NumericColumn],
    yi: usize,
    a: usize,
    b: usize,
    c: usize,
) -> Option<RawHit> {
    if looks_like_ones(&cols[a].values) || looks_like_ones(&cols[b].values) {
        return None;
    }
    if !significant_addend(&cols[yi].values, &cols[c].values) {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i], cols[c].values[i]) {
        (Some(av), Some(bv), Some(cv)) => Some(av * bv + cv),
        _ => None,
    });
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH || !residual_plausible(mae, y) {
        return None;
    }
    Some(RawHit {
        yi,
        inputs: vec![a, b, c],
        template: IdentityTemplate::ProductPlus,
        k: None,
        match_ratio,
        mae,
        pairs,
    })
}

fn try_sum3(cols: &[NumericColumn], yi: usize, a: usize, b: usize, c: usize) -> Option<RawHit> {
    if !significant_addend(&cols[yi].values, &cols[a].values)
        || !significant_addend(&cols[yi].values, &cols[b].values)
        || !significant_addend(&cols[yi].values, &cols[c].values)
    {
        return None;
    }
    let y = &cols[yi].values;
    let (pairs, hits, mae) = score_pred(y, |i| match (cols[a].values[i], cols[b].values[i], cols[c].values[i]) {
        (Some(av), Some(bv), Some(cv)) => Some(av + bv + cv),
        _ => None,
    });
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH || !residual_plausible(mae, y) {
        return None;
    }
    Some(RawHit {
        yi,
        inputs: vec![a, b, c],
        template: IdentityTemplate::Sum3,
        k: None,
        match_ratio,
        mae,
        pairs,
    })
}

fn finish_binary(
    yi: usize,
    y: &[Option<f64>],
    inputs: Vec<usize>,
    template: IdentityTemplate,
    pairs: u64,
    hits: u64,
    mae: f64,
) -> Option<RawHit> {
    let match_ratio = ratio(hits, pairs)?;
    if match_ratio < MIN_MATCH {
        return None;
    }
    if !residual_plausible(mae, y) {
        return None;
    }
    Some(RawHit {
        yi,
        inputs,
        template,
        k: None,
        match_ratio,
        mae,
        pairs,
    })
}

fn score_pred(y: &[Option<f64>], pred: impl Fn(usize) -> Option<f64>) -> (u64, u64, f64) {
    let mut pairs = 0u64;
    let mut hits = 0u64;
    let mut abs_err = 0.0;
    for i in 0..y.len() {
        let (Some(yv), Some(yhat)) = (y[i], pred(i)) else {
            continue;
        };
        if !yv.is_finite() || !yhat.is_finite() {
            continue;
        }
        pairs += 1;
        let err = (yv - yhat).abs();
        abs_err += err;
        if nearly_equal(yv, yhat) {
            hits += 1;
        }
    }
    let mae = if pairs == 0 {
        f64::INFINITY
    } else {
        abs_err / pairs as f64
    };
    (pairs, hits, mae)
}

fn nearly_equal(y: f64, yhat: f64) -> bool {
    let err = (y - yhat).abs();
    let scale = y.abs().max(yhat.abs());
    if scale <= 1.0 {
        return err <= 0.005;
    }
    if err <= 0.02 {
        return true;
    }
    let y_cents = (y * 100.0).round();
    let hat_cents = (yhat * 100.0).round();
    if y_cents == hat_cents {
        return true;
    }
    err <= 1e-4 * scale.max(1.0)
}

fn residual_plausible(mae: f64, y: &[Option<f64>]) -> bool {
    let scale = mean_abs(y).max(0.5);
    mae <= 0.15 * scale + 0.05
}

fn significant_addend(target: &[Option<f64>], addend: &[Option<f64>]) -> bool {
    let t = mean_abs(target);
    let a = mean_abs(addend);
    if t <= NEAR_ZERO {
        return false;
    }
    a > 0.04 * t
}

fn looks_like_ones(values: &[Option<f64>]) -> bool {
    let mut n = 0u64;
    let mut ones = 0u64;
    for v in values.iter().flatten() {
        n += 1;
        if (*v - 1.0).abs() <= 0.02 {
            ones += 1;
        }
    }
    n >= IDENTITY_MIN_PAIRS && ones as f64 / n as f64 >= 0.9
}

fn mean_abs(values: &[Option<f64>]) -> f64 {
    let mut n = 0u64;
    let mut s = 0.0;
    for v in values.iter().flatten() {
        s += v.abs();
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        s / n as f64
    }
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

fn snap_ratio(k: f64) -> f64 {
    let mut best = k;
    let mut best_score = (f64::INFINITY, i32::MAX);
    let scale = k.abs().max(1e-6);
    for q in 1..=1000 {
        let p = (k * q as f64).round() as i32;
        if p == 0 {
            continue;
        }
        let cand = p as f64 / q as f64;
        let rel = (cand - k).abs() / scale;
        if rel <= 5e-4 {
            let score = (rel, q);
            if score < best_score {
                best_score = score;
                best = cand;
            }
        }
    }
    best
}

fn ratio(hits: u64, pairs: u64) -> Option<f64> {
    if pairs < IDENTITY_MIN_PAIRS {
        None
    } else {
        Some(hits as f64 / pairs as f64)
    }
}

fn to_identity(sample: &NumericSample, hit: RawHit, scope: IdentityScope) -> ColumnIdentity {
    let target = col_ref(&sample.columns[hit.yi]);
    let inputs: Vec<ColumnRef> = hit
        .inputs
        .iter()
        .map(|&i| col_ref(&sample.columns[i]))
        .collect();
    let expression = format_expression(&sample.columns, &hit);
    let scale = mean_abs(&sample.columns[hit.yi].values).max(1.0);
    let residual = (1.0 - (hit.mae / scale)).clamp(0.0, 1.0);
    let complexity = match hit.template {
        IdentityTemplate::Equal | IdentityTemplate::GrainSum => 0.0,
        IdentityTemplate::Rate | IdentityTemplate::Sum | IdentityTemplate::Product => 0.03,
        IdentityTemplate::ScaledProduct | IdentityTemplate::Sum3 => 0.04,
        IdentityTemplate::Difference | IdentityTemplate::Quotient => 0.05,
        IdentityTemplate::ProductMinus | IdentityTemplate::ProductPlus => 0.08,
    };
    let confidence = (hit.match_ratio * 0.78 + residual * 0.22 - complexity).clamp(0.0, 0.99);
    let reason = format!(
        "`{expression}` holds on {:.1}% of {} sampled rows (MAE {:.4}); names were not used",
        hit.match_ratio * 100.0,
        hit.pairs,
        hit.mae
    );
    ColumnIdentity {
        id: format!(
            "{}:{}~{}",
            template_tag(hit.template),
            target.qualified(),
            inputs
                .iter()
                .map(|c| c.qualified())
                .collect::<Vec<_>>()
                .join(",")
        ),
        scope,
        target,
        inputs,
        template: hit.template,
        expression,
        match_ratio: round6(hit.match_ratio),
        mae: round6(hit.mae),
        pair_count: hit.pairs,
        coefficient: hit.k.map(round6),
        confidence: round6(confidence),
        certainty: Certainty::from_confidence(confidence),
        reason,
        target_scale: mean_abs(&sample.columns[hit.yi].values),
    }
}

fn col_ref(col: &NumericColumn) -> ColumnRef {
    ColumnRef::new(&col.table, &col.column)
}

fn term(col: &NumericColumn) -> String {
    let q = format!("{}.{}", col.table, col.column);
    if col.summed {
        format!("SUM({q})")
    } else {
        q
    }
}

fn format_expression(cols: &[NumericColumn], hit: &RawHit) -> String {
    let y = term(&cols[hit.yi]);
    let ins: Vec<String> = hit.inputs.iter().map(|&i| term(&cols[i])).collect();
    match hit.template {
        IdentityTemplate::Equal => format!("{y} ≈ {}", ins[0]),
        IdentityTemplate::Rate | IdentityTemplate::GrainSum => {
            if let Some(k) = hit.k {
                if (k - 1.0).abs() <= 0.005 {
                    format!("{y} ≈ {}", ins[0])
                } else {
                    format!("{y} ≈ {} * {}", format_k(k), ins[0])
                }
            } else {
                format!("{y} ≈ {}", ins[0])
            }
        }
        IdentityTemplate::Sum => format!("{y} ≈ {} + {}", ins[0], ins[1]),
        IdentityTemplate::Sum3 => format!("{y} ≈ {} + {} + {}", ins[0], ins[1], ins[2]),
        IdentityTemplate::Difference => format!("{y} ≈ {} - {}", ins[0], ins[1]),
        IdentityTemplate::Product => format!("{y} ≈ {} * {}", ins[0], ins[1]),
        IdentityTemplate::ScaledProduct => {
            let k = hit.k.unwrap_or(1.0);
            format!("{y} ≈ {} * {} * {}", format_k(k), ins[0], ins[1])
        }
        IdentityTemplate::Quotient => format!("{y} ≈ {} / {}", ins[0], ins[1]),
        IdentityTemplate::ProductMinus => format!("{y} ≈ {} * {} - {}", ins[0], ins[1], ins[2]),
        IdentityTemplate::ProductPlus => format!("{y} ≈ {} * {} + {}", ins[0], ins[1], ins[2]),
    }
}

fn format_k(k: f64) -> String {
    if (k * 100.0).round() / 100.0 == k || (k * 1000.0).round() / 1000.0 == k {
        let s = format!("{k:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        format!("{k:.6}")
    }
}

fn template_tag(template: IdentityTemplate) -> &'static str {
    match template {
        IdentityTemplate::Equal => "eq",
        IdentityTemplate::Rate => "rate",
        IdentityTemplate::Sum => "sum",
        IdentityTemplate::Sum3 => "sum3",
        IdentityTemplate::Difference => "diff",
        IdentityTemplate::Product => "prod",
        IdentityTemplate::ScaledProduct => "kprod",
        IdentityTemplate::Quotient => "div",
        IdentityTemplate::ProductMinus => "prod_minus",
        IdentityTemplate::ProductPlus => "prod_plus",
        IdentityTemplate::GrainSum => "grain_sum",
    }
}

fn template_rank(template: IdentityTemplate) -> u8 {
    match template {
        IdentityTemplate::Equal | IdentityTemplate::GrainSum => 0,
        IdentityTemplate::Sum | IdentityTemplate::Product => 1,
        IdentityTemplate::Rate | IdentityTemplate::Sum3 => 2,
        IdentityTemplate::ScaledProduct => 3,
        IdentityTemplate::Difference | IdentityTemplate::Quotient => 4,
        IdentityTemplate::ProductMinus | IdentityTemplate::ProductPlus => 5,
    }
}

fn family_key(id: &ColumnIdentity) -> String {
    let mut names: Vec<String> = id.inputs.iter().map(|c| c.qualified()).collect();
    names.push(id.target.qualified());
    names.sort();
    let family = match id.template {
        IdentityTemplate::Equal | IdentityTemplate::Rate => "linear",
        IdentityTemplate::Sum | IdentityTemplate::Difference => "additive",
        IdentityTemplate::Sum3 => "additive3",
        IdentityTemplate::Product | IdentityTemplate::Quotient | IdentityTemplate::ScaledProduct => {
            "multiplicative"
        }
        IdentityTemplate::ProductMinus => "product_minus",
        IdentityTemplate::ProductPlus => "product_plus",
        IdentityTemplate::GrainSum => "grain",
    };
    format!("{family}:{}", names.join("|"))
}

fn rate_form_rank(id: &ColumnIdentity) -> u8 {
    match id.coefficient {
        Some(k) if k > 0.0 && k <= 1.0 => 0,
        Some(_) => 1,
        None => 0,
    }
}

fn scope_rank(scope: IdentityScope) -> u8 {
    match scope {
        IdentityScope::IntraTable => 0,
        IdentityScope::Grain => 1,
        IdentityScope::Join => 2,
    }
}

fn dedup_hits(mut hits: Vec<ColumnIdentity>) -> Vec<ColumnIdentity> {
    hits.sort_by(|a, b| {
        b.match_ratio
            .partial_cmp(&a.match_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| template_rank(a.template).cmp(&template_rank(b.template)))
            .then_with(|| rate_form_rank(a).cmp(&rate_form_rank(b)))
            .then_with(|| scope_rank(a.scope).cmp(&scope_rank(b.scope)))
            .then_with(|| {
                b.target_scale
                    .partial_cmp(&a.target_scale)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.expression.cmp(&b.expression))
    });
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert(family_key(h)));
    let mut hits = prune_noisy_identities(hits);
    hits.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.expression.cmp(&b.expression))
    });
    hits
}

fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(table: &str, name: &str, values: Vec<Option<f64>>) -> NumericColumn {
        NumericColumn {
            table: table.to_string(),
            column: name.to_string(),
            summed: false,
            values,
        }
    }

    fn sample(columns: Vec<NumericColumn>) -> NumericSample {
        NumericSample {
            label: "t".into(),
            columns,
        }
    }

    #[test]
    fn finds_sum_product_and_rate_without_names() {
        let alpha: Vec<Option<f64>> = (1..=12).map(|i| Some(i as f64 * 2.0)).collect();
        let beta: Vec<Option<f64>> = (1..=12).map(|i| Some(i as f64 * 0.5 + 1.0)).collect();
        let gamma: Vec<Option<f64>> = alpha
            .iter()
            .zip(beta.iter())
            .map(|(a, b)| Some(a.unwrap() + b.unwrap()))
            .collect();
        let delta: Vec<Option<f64>> = alpha
            .iter()
            .zip(beta.iter())
            .map(|(a, b)| Some(a.unwrap() * b.unwrap()))
            .collect();
        let eps: Vec<Option<f64>> = alpha
            .iter()
            .map(|a| Some((a.unwrap() * 0.19 * 100.0).round() / 100.0))
            .collect();
        let found = detect_in_sample(
            &sample(vec![
                col("t", "alpha", alpha),
                col("t", "beta", beta),
                col("t", "gamma", gamma),
                col("t", "delta", delta),
                col("t", "eps", eps),
            ]),
            IdentityScope::IntraTable,
            None,
        );
        assert!(
            found.iter().any(|h| h.template == IdentityTemplate::Sum
                && h.target.column == "gamma"
                && h.match_ratio >= 0.99),
            "{found:?}"
        );
        assert!(
            found.iter().any(|h| h.template == IdentityTemplate::Product
                && h.target.column == "delta"
                && h.match_ratio >= 0.99),
            "{found:?}"
        );
        let rate = found
            .iter()
            .find(|h| h.template == IdentityTemplate::Rate && h.target.column == "eps")
            .expect("rate");
        assert!((rate.coefficient.unwrap() - 0.19).abs() < 0.002, "{rate:?}");
    }

    #[test]
    fn unrelated_columns_are_not_identities() {
        let a: Vec<Option<f64>> = (1..=20).map(|i| Some(i as f64)).collect();
        let b: Vec<Option<f64>> = (1..=20).map(|i| Some((i * 7 % 13) as f64 + 0.3)).collect();
        let found = detect_in_sample(
            &sample(vec![col("t", "a", a), col("t", "b", b)]),
            IdentityScope::IntraTable,
            None,
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn finds_scaled_product_without_names() {
        let amount: Vec<Option<f64>> = (1..=12).map(|i| Some(1000.0 * i as f64)).collect();
        let pct: Vec<Option<f64>> = [10.0, 20.0, 40.0, 60.0, 80.0, 100.0]
            .iter()
            .cycle()
            .take(12)
            .map(|v| Some(*v))
            .collect();
        let weighted: Vec<Option<f64>> = amount
            .iter()
            .zip(pct.iter())
            .map(|(a, p)| Some(((a.unwrap() * p.unwrap() / 100.0) * 100.0).round() / 100.0))
            .collect();
        let found = detect_in_sample(
            &sample(vec![
                col("t", "amount", amount),
                col("t", "pct", pct),
                col("t", "weighted", weighted),
            ]),
            IdentityScope::IntraTable,
            None,
        );
        let hit = found
            .iter()
            .find(|h| h.template == IdentityTemplate::ScaledProduct && h.target.column == "weighted")
            .expect("scaled product");
        assert!((hit.coefficient.unwrap() - 0.01).abs() < 0.001, "{hit:?}");
        assert!(hit.match_ratio >= 0.99, "{hit:?}");
    }
}
