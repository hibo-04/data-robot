use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlDialect {
    Generic,
    Postgres,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Integer,
    BigInteger,
    Float,
    Decimal,
    Boolean,
    Text,
    Date,
    Timestamp,
    Json,
    Unknown(String),
}

impl DataType {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "int" | "integer" | "int4" | "int2" | "smallint" | "serial" => Self::Integer,
            "bigint" | "int8" | "bigserial" => Self::BigInteger,
            "float" | "float4" | "float8" | "double" | "double precision" | "real" => Self::Float,
            "decimal" | "numeric" | "money" => Self::Decimal,
            "bool" | "boolean" => Self::Boolean,
            "text" | "varchar" | "character varying" | "char" | "bpchar" | "citext" | "uuid"
            | "name" => Self::Text,
            "date" => Self::Date,
            "timestamp" | "timestamptz" | "timestamp without time zone"
            | "timestamp with time zone" => Self::Timestamp,
            "json" | "jsonb" => Self::Json,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer | Self::BigInteger | Self::Float | Self::Decimal
        )
    }

    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::Date | Self::Timestamp)
    }

    pub fn is_textish(&self) -> bool {
        matches!(self, Self::Text | Self::Json | Self::Unknown(_))
    }

    /// 1.0 = identical family, 0.5 = compatible numeric, 0.0 = incompatible.
    pub fn compatibility(&self, other: &DataType) -> f64 {
        if self == other {
            return 1.0;
        }
        if self.is_numeric() && other.is_numeric() {
            return 0.85;
        }
        if self.is_temporal() && other.is_temporal() {
            return 0.9;
        }
        if self.is_textish() && other.is_textish() {
            return 0.7;
        }
        // integer-looking codes stored as text vs integer ids
        if (self.is_numeric() && other.is_textish()) || (self.is_textish() && other.is_numeric()) {
            return 0.45;
        }
        0.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Cell {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Date(NaiveDate),
    Timestamp(NaiveDateTime),
}

impl Cell {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn canonical(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Int(v) => v.to_string(),
            Self::Float(v) => {
                if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
                    format!("{}", *v as i64)
                } else {
                    format!("{v:.8}")
                }
            }
            Self::Text(v) => v.trim().to_string(),
            Self::Bool(v) => v.to_string(),
            Self::Date(v) => v.to_string(),
            Self::Timestamp(v) => v.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            Self::Text(v) => v.parse().ok(),
            Self::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Self::Date(d) => Some(*d),
            Self::Timestamp(ts) => Some(ts.date()),
            Self::Text(v) => NaiveDate::parse_from_str(v, "%Y-%m-%d")
                .ok()
                .or_else(|| NaiveDateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S").ok().map(|ts| ts.date())),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        if self.is_null() {
            "NULL".to_string()
        } else {
            self.canonical()
        }
    }
}

pub fn parse_cell(raw: &str, ty: &DataType) -> Cell {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed == "\\N" {
        return Cell::Null;
    }
    match ty {
        DataType::Integer | DataType::BigInteger => trimmed
            .parse::<i64>()
            .map(Cell::Int)
            .or_else(|_| trimmed.parse::<f64>().map(|v| Cell::Int(v as i64)))
            .unwrap_or_else(|_| Cell::Text(trimmed.to_string())),
        DataType::Float | DataType::Decimal => trimmed
            .parse::<f64>()
            .map(Cell::Float)
            .unwrap_or_else(|_| Cell::Text(trimmed.to_string())),
        DataType::Boolean => match trimmed.to_ascii_lowercase().as_str() {
            "1" | "t" | "true" | "yes" | "y" => Cell::Bool(true),
            "0" | "f" | "false" | "no" | "n" => Cell::Bool(false),
            _ => Cell::Text(trimmed.to_string()),
        },
        DataType::Date => NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
            .map(Cell::Date)
            .unwrap_or_else(|_| Cell::Text(trimmed.to_string())),
        DataType::Timestamp => NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S"))
            .map(Cell::Timestamp)
            .or_else(|_| {
                NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                    .map(|d| Cell::Timestamp(d.and_hms_opt(0, 0, 0).expect("midnight")))
            })
            .unwrap_or_else(|_| Cell::Text(trimmed.to_string())),
        DataType::Text | DataType::Json | DataType::Unknown(_) => Cell::Text(trimmed.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnRef {
    pub table: String,
    pub column: String,
}

impl ColumnRef {
    pub fn new(table: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
        }
    }

    pub fn qualified(&self) -> String {
        format!("{}.{}", self.table, self.column)
    }
}

impl std::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.qualified())
    }
}
