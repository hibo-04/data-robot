use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("{0}")]
    Message(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("fixture `{path}`: {message}")]
    Fixture { path: PathBuf, message: String },

    #[error("fixture exceeds the V1 data budget: {0}")]
    FixtureBudget(String),

    #[error("unknown table `{0}`")]
    UnknownTable(String),

    #[error("unknown column `{table}.{column}`")]
    UnknownColumn { table: String, column: String },

    #[error("query planning failed: {0}")]
    QueryPlan(String),

    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
}

pub type Result<T> = std::result::Result<T, AnalyticsError>;

impl AnalyticsError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    pub fn fixture(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Fixture {
            path: path.into(),
            message: message.into(),
        }
    }
}
