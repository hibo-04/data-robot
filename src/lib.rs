//! Source-agnostic analytics core.
//!
//! Pipeline: schema → profile → relationships → identities → semantic model → KPIs → queries → reports.
//! Connectors implement [`connector::DataSource`]; engines never import PostgreSQL.

pub mod benchmark;
pub mod connector;
pub mod dictionary;
pub mod error;
pub mod identities;
pub mod kpi;
pub mod limits;
pub mod pipeline;
pub mod profiling;
pub mod query;
pub mod relationships;
pub mod reports;
pub mod schema;
pub mod semantic;
pub mod types;

pub use connector::{DataSource, FixtureConnector, PostgresConnector};
pub use error::{AnalyticsError, Result};
pub use pipeline::{analyze, analyze_with_packs, query_from_analysis, Analysis};
