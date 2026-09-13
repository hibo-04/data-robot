use std::path::PathBuf;

use analytics::benchmark::{evaluate, load_ground_truth};
use analytics::connector::FixtureConnector;
use analytics::limits::{MAX_FIXTURE_ROWS, MAX_FIXTURE_TABLES};
use analytics::pipeline::{analyze, analyze_with_packs, query_from_analysis};
use analytics::query::SemanticQuery;
use analytics::relationships::{RelationshipBand, RelationshipSource};
use analytics::DataSource;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

#[test]
fn ecommerce_dirty_stays_within_budget() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let schema = source.schema().unwrap();
    assert!(schema.tables.len() <= MAX_FIXTURE_TABLES);
    assert!(source.row_count_total() <= MAX_FIXTURE_ROWS);
    assert!(source.row_count_total() > 1000);
}

const DOMAIN_FIXTURES: &[&str] = &[
    "erp_clean",
    "erp_dirty",
    "production_clean",
    "production_dirty",
    "logistics_clean",
    "logistics_dirty",
    "saas_clean",
    "saas_dirty",
    "projects_clean",
    "projects_dirty",
];

#[test]
fn domain_fixtures_stay_within_budget_and_have_ground_truth() {
    for name in DOMAIN_FIXTURES {
        let path = fixture(name);
        let source = FixtureConnector::open(&path).unwrap();
        let schema = source.schema().unwrap();
        assert!(
            schema.tables.len() <= MAX_FIXTURE_TABLES,
            "{name}: {} tables",
            schema.tables.len()
        );
        assert!(
            source.row_count_total() <= MAX_FIXTURE_ROWS,
            "{name}: {} rows",
            source.row_count_total()
        );
        let truth = load_ground_truth(&path).unwrap().expect(name);
        assert!(!truth.relationships.is_empty(), "{name} needs relationship truth");
        assert!(!truth.kpis.is_empty(), "{name} needs KPI truth");
    }
}

#[test]
fn domain_clean_fixtures_declare_foreign_keys() {
    for name in ["erp_clean", "production_clean", "logistics_clean", "saas_clean", "projects_clean"] {
        let source = FixtureConnector::open(fixture(name)).unwrap();
        let analysis = analyze(&source).unwrap();
        let (declared, _, _) = analysis.relationships.counts();
        assert!(declared >= 4, "{name} declared FKs={declared}");
    }
}

#[test]
fn domain_fixtures_recover_ground_truth_kpis() {
    for name in DOMAIN_FIXTURES {
        let path = fixture(name);
        let source = FixtureConnector::open(&path).unwrap();
        let analysis = analyze(&source).unwrap();
        let truth = load_ground_truth(&path).unwrap().expect(name);
        let report = evaluate(&analysis, &truth);
        assert_eq!(
            report.relationships.recall, 1.0,
            "{name} relationship fn {:?}",
            report.relationship_false_negatives
        );
        assert_eq!(
            report.kpis.recall, 1.0,
            "{name} kpi fn {:?} extras {:?}",
            report.kpi_false_negatives,
            report.kpi_false_positives
        );
        assert_eq!(
            report.kpis.precision, 1.0,
            "{name} extra KPIs {:?}",
            report.kpi_false_positives
        );
    }
}

#[test]
fn fixture_connector_ignores_ground_truth_as_table() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let names = source.schema().unwrap().table_names();
    assert!(!names.iter().any(|n| n == "ground_truth"));
}

#[test]
fn pipeline_discovers_schema_and_profiles() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.schema.tables.len(), 8);
    assert!(analysis.schema.column_count() > 80);
    let orders = analysis.profiles.table("orders").unwrap();
    assert!(orders.row_count > 1000);
    let net = analysis.profiles.column("orders", "net_amount").unwrap();
    assert!(net.avg.unwrap() > 0.0);
    assert!(net.stddev.unwrap() > 0.0);
    let customer_nr = analysis.profiles.column("orders", "customer_nr").unwrap();
    assert!(customer_nr.null_count > 0);
}

#[test]
fn infers_customer_nr_without_foreign_key() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let rel = analysis
        .relationships
        .relationships
        .iter()
        .find(|r| r.from.qualified() == "orders.customer_nr" && r.to.qualified() == "customers.id")
        .expect("expected inferred orders.customer_nr → customers.id");
    assert_eq!(rel.source, RelationshipSource::Inferred);
    assert!(rel.confidence >= 0.8, "confidence {}", rel.confidence);
    assert_eq!(rel.band, RelationshipBand::Inferred);
    assert!(rel.evidence.value_coverage > 0.9);
    assert!(rel.evidence.datatype_match > 0.8);
}

#[test]
fn declared_foreign_keys_have_confidence_one() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let declared: Vec<_> = analysis
        .relationships
        .relationships
        .iter()
        .filter(|r| r.source == RelationshipSource::DatabaseConstraint)
        .collect();
    assert!(declared.len() >= 4);
    assert!(declared.iter().all(|r| r.confidence == 1.0));
}

#[test]
fn edge_cases_do_not_link_incompatible_parent_id() {
    let source = FixtureConnector::open(fixture("edge_cases")).unwrap();
    let analysis = analyze(&source).unwrap();
    let bad = analysis.relationships.relationships.iter().any(|r| {
        r.from.qualified() == "children.parent_id" && r.to.qualified() == "decoys.parent_id"
    });
    assert!(!bad, "text decoy parent_id must not match integer children.parent_id");
    let good = analysis.relationships.accepted().any(|r| {
        r.from.qualified() == "children.parent_id" && r.to.qualified() == "parents.parent_id"
    });
    assert!(good);
}

#[test]
fn kpi_detection_structural_and_commerce_labels() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let labels: Vec<_> = analysis.kpis.iter().map(|k| k.display_name().to_string()).collect();
    assert!(labels.iter().any(|n| n == "Revenue"), "{labels:?}");
    assert!(labels.iter().any(|n| n == "Units Sold"), "{labels:?}");
    assert!(labels.iter().any(|n| n.contains("Order") && n.contains("Count")), "{labels:?}");
    assert!(labels.iter().any(|n| n == "Customer Count"), "{labels:?}");
    let revenue = analysis
        .kpis
        .iter()
        .find(|k| k.display_name() == "Revenue")
        .unwrap();
    assert_eq!(revenue.name, "Order Net Amount");
    assert_eq!(revenue.dictionary_pack.as_deref(), Some("commerce"));
    assert_eq!(revenue.source.as_ref().unwrap().qualified(), "orders.net_amount");
    assert!(revenue.confidence >= 0.9);
}

#[test]
fn generic_pack_does_not_invent_commerce_names() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze_with_packs(&source, "generic").unwrap();
    let labels: Vec<_> = analysis.kpis.iter().map(|k| k.display_name().to_string()).collect();
    assert!(!labels.iter().any(|n| n == "Units Sold"), "{labels:?}");
    assert!(!labels.iter().any(|n| n == "Revenue"), "{labels:?}");
    assert!(labels.iter().any(|n| n.contains("Quantity")), "{labels:?}");
    assert!(labels.iter().any(|n| n.contains("Net Amount")), "{labels:?}");
    let qty = analysis
        .kpis
        .iter()
        .find(|k| k.source.as_ref().map(|s| s.qualified()) == Some("order_items.quantity".into()))
        .unwrap();
    assert_eq!(qty.name, "Order_item Quantity");
    assert_eq!(qty.business_label.as_deref(), Some("Quantity"));
}

#[test]
fn commerce_pack_does_not_relabel_crm_amount_as_revenue() {
    let source = FixtureConnector::open(fixture("crm")).unwrap();
    let analysis = analyze(&source).unwrap();
    let amount = analysis
        .kpis
        .iter()
        .find(|k| k.source.as_ref().map(|s| s.column.as_str()) == Some("amount"))
        .expect("opportunity amount should be a structural measure");
    assert_ne!(amount.display_name(), "Revenue");
    assert!(amount.display_name().to_ascii_lowercase().contains("amount"));
}

#[test]
fn query_planner_joins_orders_to_customers() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let query = SemanticQuery {
        measures: vec!["revenue".into()],
        dimensions: vec!["region".into()],
        filters: vec![],
        order_by: None,
        limit: Some(5),
    };
    let (plan, sql, result) = query_from_analysis(&source, &analysis, &query).unwrap();
    assert!(sql.contains("JOIN"));
    assert!(sql.to_ascii_lowercase().contains("sum"));
    assert!(sql.to_ascii_lowercase().contains("group by"));
    assert!(!plan.joins.is_empty());
    assert!(!result.rows.is_empty());
}

#[test]
fn query_year_filter_executes() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let query: SemanticQuery = serde_json::from_str(
        r#"{
            "measures": ["revenue"],
            "dimensions": ["region"],
            "filters": [{"field": "order_date", "operator": "year_equals", "value": 2025}],
            "limit": 10
        }"#,
    )
    .unwrap();
    let (_plan, sql, result) = query_from_analysis(&source, &analysis, &query).unwrap();
    assert!(sql.contains("EXTRACT(YEAR FROM"));
    assert!(!result.rows.is_empty());
}

#[test]
fn reports_include_line_and_bar() {
    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();
    let viz: Vec<_> = analysis.reports.iter().map(|r| format!("{:?}", r.visualization)).collect();
    assert!(viz.iter().any(|v| v.contains("KpiCard")), "{viz:?}");
    assert!(viz.iter().any(|v| v.contains("LineChart")), "{viz:?}");
    assert!(viz.iter().any(|v| v.contains("BarChart")), "{viz:?}");
}

#[test]
fn benchmark_dirty_recovers_core_relationships() {
    let path = fixture("ecommerce_dirty");
    let source = FixtureConnector::open(&path).unwrap();
    let analysis = analyze(&source).unwrap();
    let truth = load_ground_truth(&path).unwrap().unwrap();
    let report = evaluate(&analysis, &truth);
    assert!(
        report.relationships.recall >= 0.75,
        "recall {} fn {:?}",
        report.relationships.recall,
        report.relationship_false_negatives
    );
    assert!(
        report.kpis.recall >= 0.75,
        "kpi recall {} fn {:?}",
        report.kpis.recall,
        report.kpi_false_negatives
    );
}

#[test]
fn ecommerce_clean_uses_declared_keys() {
    let source = FixtureConnector::open(fixture("ecommerce_clean")).unwrap();
    let analysis = analyze(&source).unwrap();
    let (declared, _inferred, _) = analysis.relationships.counts();
    assert!(declared >= 3);
}

#[test]
fn crm_infers_account_nr() {
    let source = FixtureConnector::open(fixture("crm")).unwrap();
    let analysis = analyze(&source).unwrap();
    let found = analysis.relationships.accepted().any(|r| {
        r.from.qualified() == "contacts.account_nr" && r.to.qualified() == "accounts.account_id"
    });
    assert!(found, "{:?}", analysis.relationships.relationships);
}

#[test]
fn postgres_type_mapping() {
    use analytics::connector::map_postgres_type;
    use analytics::types::DataType;
    assert_eq!(map_postgres_type("int4"), DataType::Integer);
    assert_eq!(map_postgres_type("numeric"), DataType::Decimal);
    assert_eq!(map_postgres_type("timestamptz"), DataType::Timestamp);
}

#[test]
fn json_table_loads_in_edge_cases() {
    let source = FixtureConnector::open(fixture("edge_cases")).unwrap();
    let schema = source.schema().unwrap();
    assert!(schema.table("decoys").is_some());
    let analysis = analyze(&source).unwrap();
    assert_eq!(analysis.profiles.table("empty_things").unwrap().row_count, 0);
    assert_eq!(analysis.profiles.column("messy", "maybe_value").unwrap().null_ratio, 1.0);
}

#[test]
fn identities_from_values_not_names() {
    use analytics::identities::IdentityTemplate;

    let source = FixtureConnector::open(fixture("edge_cases")).unwrap();
    let analysis = analyze(&source).unwrap();

    let sum = analysis
        .identities
        .iter()
        .find(|i| i.template == IdentityTemplate::Sum && i.target.qualified() == "computed.gamma")
        .expect("gamma ≈ alpha + beta");
    assert!(sum.match_ratio >= 0.99, "{sum:?}");
    assert!(sum.inputs.iter().any(|c| c.column == "alpha"));
    assert!(sum.inputs.iter().any(|c| c.column == "beta"));
    assert!(sum.reason.contains("names were not used"));

    let product = analysis
        .identities
        .iter()
        .find(|i| i.template == IdentityTemplate::Product && i.target.column == "delta")
        .expect("delta ≈ alpha * beta");
    assert!(product.match_ratio >= 0.99, "{product:?}");

    let rate = analysis
        .identities
        .iter()
        .find(|i| i.template == IdentityTemplate::Rate && i.target.column == "eps")
        .expect("eps ≈ 0.19 * alpha");
    assert!((rate.coefficient.unwrap() - 0.19).abs() < 0.002, "{rate:?}");

    let grain = analysis
        .identities
        .iter()
        .find(|i| i.template == IdentityTemplate::GrainSum && i.target.qualified() == "bundles.span")
        .expect("span ≈ SUM(width)");
    assert!(grain.match_ratio >= 0.99, "{grain:?}");
    assert!(grain.inputs.iter().any(|c| c.qualified() == "pieces.width"));
}

#[test]
fn ecommerce_identities_find_invoice_math_without_keywords() {
    use analytics::identities::IdentityTemplate;

    let source = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&source).unwrap();

    let rate = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::Rate
            && i.match_ratio >= 0.95
            && ((i.target.column == "gross_amount" && i.inputs.iter().any(|c| c.column == "tax_amount"))
                || (i.target.column == "tax_amount" && i.inputs.iter().any(|c| c.column == "gross_amount")))
    });
    assert!(rate, "expected a rate between invoice gross and tax: {:?}", analysis.identities);

    let join_sum = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::Sum
            && i.target.column == "gross_amount"
            && i.inputs.iter().any(|c| c.column == "net_amount")
            && i.inputs.iter().any(|c| c.column == "tax_amount")
            && i.match_ratio >= 0.95
    });
    assert!(join_sum, "expected gross ≈ net + tax: {:?}", analysis.identities);

    let line_total = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::ProductMinus
            && i.target.qualified() == "order_items.line_total"
            && i.match_ratio >= 0.95
    }) || analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::Product
            && i.target.qualified() == "order_items.line_total"
            && i.match_ratio >= 0.95
    });
    assert!(line_total, "expected line_total ≈ qty * price - discount: {:?}", analysis.identities);

    let remaining = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::Difference
            && i.target.column == "remaining_amount"
            && i.inputs.iter().any(|c| c.column == "gross_amount")
            && i.inputs.iter().any(|c| c.column == "paid_amount")
            && i.match_ratio >= 0.95
    }) || analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::Sum
            && i.match_ratio >= 0.95
            && i.inputs.iter().any(|c| c.column == "remaining_amount")
            && i.inputs.iter().any(|c| c.column == "paid_amount")
            && i.target.column == "gross_amount"
    });
    assert!(remaining, "expected remaining ≈ gross - paid: {:?}", analysis.identities);

    let false_grain = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::GrainSum
            && i.target.qualified() == "orders.net_amount"
            && i.inputs.iter().any(|c| c.qualified() == "order_items.quantity")
    });
    assert!(!false_grain, "net_amount must not reconstruct from SUM(quantity)");
}

#[test]
fn identities_find_scaled_product_in_crm_and_weight() {
    use analytics::identities::IdentityTemplate;

    let crm = FixtureConnector::open(fixture("crm")).unwrap();
    let analysis = analyze(&crm).unwrap();
    let weighted = analysis.identities.iter().find(|i| {
        i.template == IdentityTemplate::ScaledProduct
            && i.target.column == "weighted_amount"
            && i.inputs.iter().any(|c| c.column == "amount")
            && i.inputs.iter().any(|c| c.column == "probability")
    });
    assert!(
        weighted.is_some(),
        "weighted_amount ≈ k * amount * probability: {:?}",
        analysis.identities
    );
    let weighted = weighted.unwrap();
    assert!((weighted.coefficient.unwrap() - 0.01).abs() < 0.002, "{weighted:?}");
    assert!(weighted.match_ratio >= 0.95, "{weighted:?}");

    let shop = FixtureConnector::open(fixture("ecommerce_dirty")).unwrap();
    let analysis = analyze(&shop).unwrap();
    let weight = analysis.identities.iter().any(|i| {
        i.template == IdentityTemplate::ScaledProduct
            && i.target.column == "weight_g"
            && i.match_ratio >= 0.95
            && i.inputs.iter().any(|c| c.column == "weight_kg")
            && i.inputs.iter().any(|c| c.column == "quantity")
            && i.coefficient.map(|k| (k - 1000.0).abs() < 1.0).unwrap_or(false)
    });
    assert!(weight, "expected weight_g ≈ 1000 * kg * qty: {:?}", analysis.identities);
}
