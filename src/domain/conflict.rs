use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A single cell-level conflict detected by the 3-way merge.
///
/// A conflict exists when the same `(table, pk, column)` triple was modified
/// **both** in the source (source) AND in target (target) since the base
/// snapshot was taken at clone time. Both values differ from the base, and
/// they differ from each other — so there is no safe automatic resolution.
///
/// The external orchestrator / back-office presents these to the admin who
/// chooses which value to keep (source, target, or a custom value).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConflictReport {
    /// Table where the conflict was found.
    pub table_name: String,

    /// Primary key identifying the conflicting row.
    pub pk: BTreeMap<String, Value>,

    /// Column whose value conflicts.
    pub column: String,

    /// Value in the base snapshot (target at source-clone time).
    pub base_value: Value,

    /// Value in the source (what the admin changed).
    pub source_value: Value,

    /// Current value in target (what another admin deployed since the clone).
    pub target_value: Value,
}
