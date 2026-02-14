use serde::{Deserialize, Serialize};

/// Newtype to avoid confusion between schema names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Schema(pub String);

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
