use serde::{Deserialize, Serialize};

/// Newtype to avoid confusion between schema names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Schema(pub String);

/// SHA-256 hex fingerprint of a table's canonical row content.
///
/// Computed by `diffly::fingerprint(rows)`. Stored externally (S3, DynamoDB)
/// by the orchestrator at source-clone time and passed back to diffly at
/// deploy time so `ConflictService` can detect concurrent target changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint(pub String);

impl Fingerprint {
    /// Returns the raw hex string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Newtype for table names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableName(pub String);

/// Newtype for column names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ColumnName(pub String);

/// List of columns to exclude from the diff (e.g., created_at, updated_at)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExcludedColumns(pub Vec<String>);

impl ExcludedColumns {
    pub fn contains(&self, col: &str) -> bool {
        self.0.iter().any(|c| c == col)
    }
}
