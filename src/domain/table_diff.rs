use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Type alias for a database row represented as a sorted map of column name → JSON value.
pub type RowMap = BTreeMap<String, Value>;

#[derive(Debug, Serialize, Clone)]
pub struct TableDiff {
    pub table_name: String,
    pub primary_key: Vec<String>,
    pub inserts: Vec<RowChange>,
    pub updates: Vec<RowUpdate>,
    pub deletes: Vec<RowChange>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RowChange {
    pub pk: BTreeMap<String, Value>,
    pub data: RowMap,
}

#[derive(Debug, Serialize, Clone)]
pub struct RowUpdate {
    pub pk: BTreeMap<String, Value>,
    pub before: RowMap,
    pub after: RowMap,
    pub changed_columns: Vec<ColumnDiff>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ColumnDiff {
    pub column: String,
    pub before: Value,
    pub after: Value,
}

impl TableDiff {
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.updates.is_empty() && self.deletes.is_empty()
    }
}
