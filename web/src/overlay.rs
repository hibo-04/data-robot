use std::fs;
use std::path::{Path, PathBuf};

use analytics::kpi::KpiCandidate;
use analytics::pipeline::Analysis;
use analytics::relationships::{Relationship, RelationshipBand};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Versioned user decisions on top of the last analysis. Matches `docs/model-review.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overlay {
    pub source_id: String,
    pub updated_at: String,
    #[serde(default)]
    pub relationships: Vec<RelationshipDecision>,
    #[serde(default)]
    pub kpis: Vec<KpiDecision>,
    #[serde(default)]
    pub identities: Vec<IdentityDecision>,
    #[serde(default)]
    pub packs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDecision {
    pub from: String,
    pub to: String,
    pub status: ReviewStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiDecision {
    pub id: String,
    pub status: ReviewStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDecision {
    pub id: String,
    pub status: ReviewStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Accepted,
    Rejected,
    Edited,
    Pending,
}

impl ReviewStatus {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "edited" => Some(Self::Edited),
            "pending" => Some(Self::Pending),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Edited => "edited",
            Self::Pending => "pending",
        }
    }

    pub fn is_rejected(self) -> bool {
        matches!(self, Self::Rejected)
    }
}

impl Overlay {
    pub fn empty(source_id: impl Into<String>, packs: &[String]) -> Self {
        Self {
            source_id: source_id.into(),
            updated_at: now(),
            relationships: Vec::new(),
            kpis: Vec::new(),
            identities: Vec::new(),
            packs: packs.to_vec(),
        }
    }

    pub fn path(root: &Path, source_id: &str) -> PathBuf {
        root.join(format!("{source_id}.json"))
    }

    pub fn load(root: &Path, source_id: &str, packs: &[String]) -> Self {
        let path = Self::path(root, source_id);
        match fs::read_to_string(&path) {
            Ok(text) => {
                serde_json::from_str(&text).unwrap_or_else(|_| Self::empty(source_id, packs))
            }
            Err(_) => Self::empty(source_id, packs),
        }
    }

    pub fn save(&self, root: &Path) -> analytics::Result<()> {
        fs::create_dir_all(root)?;
        let path = Self::path(root, &self.source_id);
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn relationship_status(&self, rel: &Relationship) -> ReviewStatus {
        let from = rel.from.qualified();
        let to = rel.to.qualified();
        if let Some(d) = self
            .relationships
            .iter()
            .find(|d| d.from == from && d.to == to)
        {
            return d.status;
        }
        match rel.band {
            RelationshipBand::Declared => ReviewStatus::Accepted,
            RelationshipBand::Inferred | RelationshipBand::Uncertain => ReviewStatus::Pending,
        }
    }

    pub fn kpi_status(&self, id: &str) -> ReviewStatus {
        self.kpis
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.status)
            .unwrap_or(ReviewStatus::Pending)
    }

    pub fn kpi_label<'a>(&'a self, id: &str) -> Option<&'a str> {
        self.kpis
            .iter()
            .find(|d| d.id == id)
            .and_then(|d| d.label.as_deref())
            .filter(|s| !s.trim().is_empty())
    }

    pub fn identity_status(&self, id: &str) -> ReviewStatus {
        self.identities
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.status)
            .unwrap_or(ReviewStatus::Pending)
    }

    pub fn set_relationship(&mut self, from: &str, to: &str, status: ReviewStatus) {
        if let Some(d) = self
            .relationships
            .iter_mut()
            .find(|d| d.from == from && d.to == to)
        {
            d.status = status;
        } else {
            self.relationships.push(RelationshipDecision {
                from: from.to_string(),
                to: to.to_string(),
                status,
                user_note: None,
            });
        }
        self.touch();
    }

    pub fn set_kpi(&mut self, id: &str, status: ReviewStatus, label: Option<String>) {
        if let Some(d) = self.kpis.iter_mut().find(|d| d.id == id) {
            d.status = status;
            if let Some(label) = label {
                d.label = nonempty(label);
            }
        } else {
            self.kpis.push(KpiDecision {
                id: id.to_string(),
                status,
                label: label.and_then(nonempty),
            });
        }
        self.touch();
    }

    pub fn set_identity(&mut self, id: &str, status: ReviewStatus) {
        if let Some(d) = self.identities.iter_mut().find(|d| d.id == id) {
            d.status = status;
        } else {
            self.identities.push(IdentityDecision {
                id: id.to_string(),
                status,
            });
        }
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = now();
    }
}

/// Analysis with rejected items removed and KPI labels applied. Review screens
/// still use the raw analysis so rejected rows remain visible and reversible.
pub fn publish(analysis: &Analysis, overlay: &Overlay) -> Analysis {
    let mut published = analysis.clone();
    published
        .relationships
        .relationships
        .retain(|r| !overlay.relationship_status(r).is_rejected());
    published.kpis = apply_kpis(&analysis.kpis, overlay);
    published
        .identities
        .retain(|i| !overlay.identity_status(&i.id).is_rejected());
    published.semantic_model.relationships.retain(|r| {
        overlay
            .relationships
            .iter()
            .find(|d| d.from == r.from_column.qualified() && d.to == r.to_column.qualified())
            .map(|d| !d.status.is_rejected())
            .unwrap_or(true)
    });
    published.reports.retain(|rep| match &rep.measure {
        None => true,
        Some(measure) => analysis
            .kpis
            .iter()
            .find(|k| k.matches_name(measure))
            .map(|k| !overlay.kpi_status(&k.id).is_rejected())
            .unwrap_or(true),
    });
    published
}

pub fn apply_kpis(kpis: &[KpiCandidate], overlay: &Overlay) -> Vec<KpiCandidate> {
    kpis.iter()
        .filter_map(|kpi| {
            if overlay.kpi_status(&kpi.id).is_rejected() {
                return None;
            }
            let mut kpi = kpi.clone();
            if let Some(label) = overlay.kpi_label(&kpi.id) {
                kpi.business_label = Some(label.to_string());
            }
            Some(kpi)
        })
        .collect()
}

pub fn display_kpi_name(kpi: &KpiCandidate, overlay: &Overlay) -> String {
    overlay
        .kpi_label(&kpi.id)
        .map(str::to_string)
        .unwrap_or_else(|| kpi.display_name().to_string())
}

pub fn queryable(kpi: &KpiCandidate) -> bool {
    kpi.source.is_some() && kpi.aggregation.is_some()
}

fn nonempty(label: String) -> Option<String> {
    let trimmed = label.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use analytics::relationships::{RelationshipEvidence, RelationshipSource, RelationshipType};
    use analytics::types::ColumnRef;

    fn rel(from: &str, to: &str, band: RelationshipBand) -> Relationship {
        let (ft, fc) = from.split_once('.').unwrap();
        let (tt, tc) = to.split_once('.').unwrap();
        Relationship {
            from: ColumnRef::new(ft, fc),
            to: ColumnRef::new(tt, tc),
            relationship_type: RelationshipType::ManyToOne,
            confidence: 1.0,
            source: RelationshipSource::DatabaseConstraint,
            band,
            evidence: RelationshipEvidence {
                datatype_match: 1.0,
                name_similarity: 1.0,
                target_unique: true,
                target_is_primary_key: true,
                value_coverage: 1.0,
                reverse_coverage: 0.0,
                from_null_ratio: 0.0,
                truncated_values: false,
                notes: vec![],
            },
            reason: "test".into(),
        }
    }

    #[test]
    fn declared_edges_start_accepted() {
        let overlay = Overlay::empty("shop", &[]);
        let edge = rel(
            "orders.customer_id",
            "customers.id",
            RelationshipBand::Declared,
        );
        assert_eq!(overlay.relationship_status(&edge), ReviewStatus::Accepted);
    }

    #[test]
    fn inferred_edges_start_pending() {
        let overlay = Overlay::empty("shop", &[]);
        let edge = rel(
            "orders.customer_nr",
            "customers.id",
            RelationshipBand::Inferred,
        );
        assert_eq!(overlay.relationship_status(&edge), ReviewStatus::Pending);
    }

    #[test]
    fn reject_survives_reload_shape() {
        let mut overlay = Overlay::empty("ecommerce_dirty", &["generic".into(), "commerce".into()]);
        overlay.set_relationship("orders.customer_nr", "customers.id", ReviewStatus::Accepted);
        overlay.set_kpi(
            "sum:order_items.quantity",
            ReviewStatus::Accepted,
            Some("Units Sold".into()),
        );
        overlay.set_identity(
            "sum:invoices.gross_amount~orders.net_amount,invoices.tax_amount",
            ReviewStatus::Rejected,
        );
        let json = serde_json::to_value(&overlay).unwrap();
        assert_eq!(json["source_id"], "ecommerce_dirty");
        assert_eq!(json["relationships"][0]["status"], "accepted");
        assert_eq!(json["kpis"][0]["label"], "Units Sold");
        assert_eq!(json["identities"][0]["status"], "rejected");

        let round: Overlay = serde_json::from_value(json).unwrap();
        assert_eq!(
            round.kpi_status("sum:order_items.quantity"),
            ReviewStatus::Accepted
        );
        assert_eq!(
            round
                .identity_status("sum:invoices.gross_amount~orders.net_amount,invoices.tax_amount"),
            ReviewStatus::Rejected
        );
    }

    #[test]
    fn docs_example_deserializes() {
        let json = r#"{
          "source_id": "ecommerce_dirty",
          "updated_at": "2026-01-01T00:00:00Z",
          "relationships": [
            {"from": "orders.customer_nr", "to": "customers.id", "status": "accepted", "user_note": "legacy customer number"},
            {"from": "returns.return_id", "to": "orders.order_id", "status": "rejected"}
          ],
          "kpis": [
            {"id": "sum:order_items.quantity", "status": "accepted", "label": "Units Sold"},
            {"id": "sum:products.list_price", "status": "rejected"}
          ],
          "identities": [
            {"id": "sum:invoices.gross_amount~orders.net_amount,invoices.tax_amount", "status": "accepted"},
            {"id": "rate:products.list_price~products.unit_cost", "status": "rejected"}
          ],
          "packs": ["generic", "commerce"]
        }"#;
        let overlay: Overlay = serde_json::from_str(json).unwrap();
        assert_eq!(overlay.kpis.len(), 2);
        assert_eq!(
            overlay.kpi_label("sum:order_items.quantity"),
            Some("Units Sold")
        );
        let rejected = rel(
            "returns.return_id",
            "orders.order_id",
            RelationshipBand::Inferred,
        );
        assert!(overlay.relationship_status(&rejected).is_rejected());
    }
}
