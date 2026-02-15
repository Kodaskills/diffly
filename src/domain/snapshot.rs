use std::collections::BTreeMap;

use crate::domain::ports::SnapshotProvider;
use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::TableName;

/// In-memory implementation of [`SnapshotProvider`].
///
/// Wraps the `BTreeMap<table_name, Vec<RowMap>>` returned by
/// `diffly::snapshot()` and makes it usable as a `SnapshotProvider` for
/// `diffly::run_with_conflicts()`.
/// ```
pub struct MapSnapshotProvider(BTreeMap<String, Vec<RowMap>>);

impl MapSnapshotProvider {
    pub fn new(data: BTreeMap<String, Vec<RowMap>>) -> Self {
        Self(data)
    }
}

impl SnapshotProvider for MapSnapshotProvider {
    fn get(&self, table: &TableName) -> Option<&[RowMap]> {
        self.0.get(&table.0).map(|v| v.as_slice())
    }
}
