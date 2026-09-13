//! Hard budgets for the fixture-based development path.
//!
//! Do not raise these constants unless the user explicitly asks to scale test data.
//! Mass-data performance work is a later phase.

/// Maximum tables allowed in a single fixture.
pub const MAX_FIXTURE_TABLES: usize = 10;

/// Maximum total rows across all tables of a single fixture.
pub const MAX_FIXTURE_ROWS: u64 = 10_000;

/// Maximum distinct values pulled per column for overlap scoring.
pub const MAX_DISTINCT_FOR_OVERLAP: usize = 20_000;

/// Maximum values retained per numeric column while computing a median.
pub const MEDIAN_VALUE_LIMIT: usize = 10_000;

/// Top-N frequent values stored on a profile.
pub const TOP_VALUES: usize = 10;

/// Rows (or parent groups) drawn when searching for numeric identities.
pub const IDENTITY_SAMPLE_ROWS: usize = 2_000;

/// Minimum paired non-null rows before a formula is considered.
pub const IDENTITY_MIN_PAIRS: u64 = 8;
