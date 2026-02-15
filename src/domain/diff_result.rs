use serde::Serialize;

use crate::domain::changeset::Changeset;
use crate::domain::conflict::ConflictReport;

/// The outcome of a conflict-aware diff run (produced by `ConflictService`).
///
/// Using an enum forces every caller to explicitly handle the conflicted case
/// at compile time — it cannot be silently ignored.
///
/// `DiffService` always produces a plain `Changeset` (2-way diff, no conflict
/// awareness). `ConflictService` wraps that changeset and enriches it with
/// conflict information when a 3-way merge reveals concurrent changes.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DiffResult {
    /// No concurrent target changes detected — the changeset can be applied
    /// directly. The Step Function proceeds to `AwaitApproval`.
    Clean(Changeset),

    /// Concurrent target changes were detected. Some or all of them conflict
    /// with source changes on the same rows/columns.
    ///
    /// The `changeset` reflects the auto-merged rows (non-conflicting changes
    /// are already resolved). The `conflicts` list must be resolved by the
    /// admin before the changeset can be applied. The Step Function routes to
    /// `CONFLICT_RESOLUTION`.
    Conflicted {
        changeset: Changeset,
        conflicts: Vec<ConflictReport>,
    },
}

impl DiffResult {
    /// Extract the inner changeset regardless of conflict status.
    /// Useful for output writers (JSON/SQL/HTML) that work on the changeset only.
    pub fn changeset(&self) -> &Changeset {
        match self {
            DiffResult::Clean(cs) => cs,
            DiffResult::Conflicted { changeset, .. } => changeset,
        }
    }

    /// Returns `true` if the result has no conflicts.
    pub fn is_clean(&self) -> bool {
        matches!(self, DiffResult::Clean(_))
    }

    /// Returns the conflicts slice (empty if clean).
    pub fn conflicts(&self) -> &[ConflictReport] {
        match self {
            DiffResult::Clean(_) => &[],
            DiffResult::Conflicted { conflicts, .. } => conflicts,
        }
    }
}
