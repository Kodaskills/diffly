use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::changeset::Changeset;
use crate::domain::conflict::ConflictReport;
use crate::domain::diff_result::DiffResult;
use crate::domain::fingerprint::fingerprint;
use crate::domain::ports::SnapshotProvider;
use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::{ColumnName, Fingerprint, TableName};
use crate::infrastructure::db::sql_utils::pk_key;

// ─────────────────────────────────────────────────────────────────────────────
// ConflictService
// ─────────────────────────────────────────────────────────────────────────────

/// Performs a 3-way merge between the source changeset and the base snapshot
/// to detect rows that were concurrently modified in target.
///
/// # Responsibility (SRP)
/// `DiffService` computes the 2-way diff (source vs. current target).
/// `ConflictService` takes that result and enriches it with conflict
/// information by comparing against the base snapshot. Each service has one
/// reason to change.
///
/// # Algorithm
/// For each table in the changeset:
/// 1. Look up the base snapshot rows (target at clone time) from the provider.
///    If absent → skip (no base to compare against → treat as clean).
/// 2. Build delta maps:
///    - `base→source`: what the admin changed in the source.
///    - `base→target`: what others deployed into target since the clone.
/// 3. For each `(pk_key, column)` present in **both** deltas where:
///    - the source value ≠ base value  (admin changed it)
///    - the target value ≠ base value  (someone else changed it)
///    - source value ≠ target value    (they chose different values)
///    → emit a `ConflictReport`.
/// 4. Auto-merged changes (only one side changed) require no action.
pub struct ConflictService;

impl ConflictService {
    pub fn new() -> Self {
        Self
    }

    /// Run the conflict check.
    ///
    /// `changeset`   — 2-way diff produced by `DiffService` (source vs. target now).
    /// `base`        — snapshot provider holding target-at-clone-time rows.
    /// `stored_fp`   — per-table fingerprints stored at clone time, used as a fast
    ///                 pre-check before doing the expensive 3-way merge. If the
    ///                 current target fingerprint matches the stored one the table
    ///                 is clean without needing a full row-by-row comparison.
    /// `current_target_rows` — the raw target rows per table (needed to recompute
    ///                 the current fingerprint and to build the base→target delta).
    pub fn check(
        &self,
        changeset: Changeset,
        base: &dyn SnapshotProvider,
        stored_fingerprints: &BTreeMap<String, Fingerprint>,
        current_target_rows: &BTreeMap<String, Vec<RowMap>>,
        pk_cols_by_table: &BTreeMap<String, Vec<ColumnName>>,
    ) -> DiffResult {
        let mut all_conflicts: Vec<ConflictReport> = Vec::new();

        for table_diff in &changeset.tables {
            let table_name = TableName(table_diff.table_name.clone());
            let pk_cols = match pk_cols_by_table.get(&table_diff.table_name) {
                Some(cols) => cols,
                None => continue,
            };

            // Fast path: compare fingerprints first.
            // If current target fingerprint == stored fingerprint, target has
            // not changed since the clone — no conflict possible for this table.
            if let (Some(stored_fp), Some(current_rows)) = (
                stored_fingerprints.get(&table_diff.table_name),
                current_target_rows.get(&table_diff.table_name),
            ) {
                let current_fp = fingerprint(current_rows);
                if &current_fp == stored_fp {
                    continue; // target unchanged for this table — clean
                }
            }

            // Slow path: 3-way merge needed.
            let base_rows = match base.get(&table_name) {
                Some(rows) => rows,
                None => continue, // no base snapshot → skip
            };
            let current_rows = match current_target_rows.get(&table_diff.table_name) {
                Some(rows) => rows,
                None => continue,
            };

            // Build indexed maps keyed by pk_key string.
            let base_index: BTreeMap<String, &RowMap> =
                base_rows.iter().map(|r| (pk_key(r, pk_cols), r)).collect();
            let current_index: BTreeMap<String, &RowMap> = current_rows
                .iter()
                .map(|r| (pk_key(r, pk_cols), r))
                .collect();

            // Build source (source) index from the changeset inserts + updates.
            // For conflict detection we only need rows that exist in source.
            // We reconstruct the full source row from the changeset's after/data fields.
            let mut source_index: BTreeMap<String, RowMap> = BTreeMap::new();
            for ins in &table_diff.inserts {
                let k = pk_key(&ins.data, pk_cols);
                source_index.insert(k, ins.data.clone());
            }
            for upd in &table_diff.updates {
                let k = pk_key(&upd.after, pk_cols);
                source_index.insert(k, upd.after.clone());
            }

            // Iterate only over rows that the source actually changed.
            // A conflict requires the source to have modified a row; rows that
            // were only changed in target (with no source counterpart) are
            // auto-merged — they cannot conflict with source changes.
            for pk_str in source_index.keys() {
                // Normalise: all three are `Option<&RowMap>`.
                // base_index / current_index store `&RowMap` values so `.get()`
                // would return `Option<&&RowMap>`; `.copied()` flattens one `&`.
                let base_row: Option<&RowMap> = base_index.get(pk_str).copied();
                let current_row: Option<&RowMap> = current_index.get(pk_str).copied();
                let source_row: Option<&RowMap> = source_index.get(pk_str).map(|r| r as &RowMap);

                // Collect all columns across all three states.
                let all_cols: BTreeSet<String> = [base_row, current_row, source_row]
                    .iter()
                    .filter_map(|opt: &Option<&RowMap>| {
                        opt.map(|r: &RowMap| r.keys().cloned().collect::<Vec<_>>())
                    })
                    .flatten()
                    .collect();

                for col in &all_cols {
                    let null = Value::Null;
                    let base_val = base_row.and_then(|r| r.get(col.as_str())).unwrap_or(&null);
                    let current_val = current_row
                        .and_then(|r| r.get(col.as_str()))
                        .unwrap_or(&null);
                    let source_val = source_row
                        .and_then(|r| r.get(col.as_str()))
                        .unwrap_or(&null);

                    let target_changed = current_val != base_val;
                    let source_changed = source_val != base_val;

                    if target_changed && source_changed && source_val != current_val {
                        // Reconstruct the PK map for the report.
                        let pk_map: BTreeMap<String, Value> = pk_cols
                            .iter()
                            .filter_map(|c| {
                                base_row
                                    .or(current_row)
                                    .and_then(|r| r.get(c.0.as_str()))
                                    .map(|v| (c.0.clone(), v.clone()))
                            })
                            .collect();

                        all_conflicts.push(ConflictReport {
                            table_name: table_diff.table_name.clone(),
                            pk: pk_map,
                            column: col.clone(),
                            base_value: base_val.clone(),
                            source_value: source_val.clone(),
                            target_value: current_val.clone(),
                        });
                    }
                }
            }
        }

        if all_conflicts.is_empty() {
            DiffResult::Clean(changeset)
        } else {
            DiffResult::Conflicted {
                changeset,
                conflicts: all_conflicts,
            }
        }
    }
}

impl Default for ConflictService {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::changeset::Changeset;
    use crate::domain::table_diff::{ColumnDiff, RowUpdate, TableDiff};
    use serde_json::json;

    // ── Helper: minimal snapshot provider backed by a BTreeMap ──

    struct MapSnapshot(BTreeMap<String, Vec<RowMap>>);

    impl SnapshotProvider for MapSnapshot {
        fn get(&self, table: &TableName) -> Option<&[RowMap]> {
            self.0.get(&table.0).map(|v| v.as_slice())
        }
    }

    fn row(pairs: &[(&str, Value)]) -> RowMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn pk_col(s: &str) -> ColumnName {
        ColumnName(s.to_string())
    }

    fn empty_changeset() -> Changeset {
        Changeset::new("source", "target", "postgres", vec![])
    }

    // ── Tests ──

    #[test]
    fn clean_when_no_base_snapshot() {
        let svc = ConflictService::new();
        let cs = empty_changeset();
        let base = MapSnapshot(BTreeMap::new());
        let result = svc.check(
            cs,
            &base,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert!(result.is_clean());
    }

    #[test]
    fn clean_when_fingerprints_match() {
        let svc = ConflictService::new();

        // target unchanged since clone
        let target_rows = vec![row(&[("id", json!(1)), ("val", json!("x"))])];
        let stored_fp = fingerprint(&target_rows);

        let table = "t";
        let cs = Changeset::new(
            "s",
            "target",
            "postgres",
            vec![TableDiff {
                table_name: table.to_string(),
                primary_key: vec!["id".to_string()],
                inserts: vec![],
                updates: vec![],
                deletes: vec![],
            }],
        );

        let base = MapSnapshot([(table.to_string(), target_rows.clone())].into());
        let stored_fps = [(table.to_string(), stored_fp)].into();
        let current_rows = [(table.to_string(), target_rows)].into();
        let pk_map = [(table.to_string(), vec![pk_col("id")])].into();

        let result = svc.check(cs, &base, &stored_fps, &current_rows, &pk_map);
        assert!(result.is_clean());
    }

    #[test]
    fn detects_conflict_on_same_row_same_column() {
        let svc = ConflictService::new();
        let table = "pricing_rules";

        // Base: discount_rate = 0.10
        let base_rows = vec![row(&[("id", json!(1)), ("discount_rate", json!(0.10))])];

        // source update: discount_rate = 0.20
        let source_after = row(&[("id", json!(1)), ("discount_rate", json!(0.20))]);

        // target (concurrent): discount_rate = 0.15
        let target_rows = vec![row(&[("id", json!(1)), ("discount_rate", json!(0.15))])];

        // Stored fingerprint is from base (different from current target)
        let stored_fp = fingerprint(&base_rows);

        let cs = Changeset::new(
            "source",
            "target",
            "postgres",
            vec![TableDiff {
                table_name: table.to_string(),
                primary_key: vec!["id".to_string()],
                inserts: vec![],
                updates: vec![RowUpdate {
                    pk: [("id".to_string(), json!(1))].into(),
                    before: row(&[("id", json!(1)), ("discount_rate", json!(0.15))]),
                    after: source_after,
                    changed_columns: vec![ColumnDiff {
                        column: "discount_rate".to_string(),
                        before: json!(0.15),
                        after: json!(0.20),
                    }],
                }],
                deletes: vec![],
            }],
        );

        let base = MapSnapshot([(table.to_string(), base_rows.clone())].into());
        let stored_fps = [(table.to_string(), stored_fp)].into();
        let current_rows = [(table.to_string(), target_rows)].into();
        let pk_map = [(table.to_string(), vec![pk_col("id")])].into();

        let result = svc.check(cs, &base, &stored_fps, &current_rows, &pk_map);
        assert!(!result.is_clean());

        let conflicts = result.conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].column, "discount_rate");
        assert_eq!(conflicts[0].base_value, json!(0.10));
        assert_eq!(conflicts[0].source_value, json!(0.20));
        assert_eq!(conflicts[0].target_value, json!(0.15));
    }

    #[test]
    fn no_conflict_when_different_rows_changed() {
        let svc = ConflictService::new();
        let table = "rules";

        // Base: two rows
        let base_rows = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
        ];

        // source updates row 1
        let source_after = row(&[("id", json!(1)), ("val", json!("source"))]);

        // target updates row 2 only
        let target_rows = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]), // row 1 unchanged
            row(&[("id", json!(2)), ("val", json!("target"))]), // row 2 changed
        ];

        let stored_fp = fingerprint(&base_rows); // differs from current target

        let cs = Changeset::new(
            "s",
            "t",
            "postgres",
            vec![TableDiff {
                table_name: table.to_string(),
                primary_key: vec!["id".to_string()],
                inserts: vec![],
                updates: vec![RowUpdate {
                    pk: [("id".to_string(), json!(1))].into(),
                    before: row(&[("id", json!(1)), ("val", json!("a"))]),
                    after: source_after,
                    changed_columns: vec![ColumnDiff {
                        column: "val".to_string(),
                        before: json!("a"),
                        after: json!("source"),
                    }],
                }],
                deletes: vec![],
            }],
        );

        let base = MapSnapshot([(table.to_string(), base_rows.clone())].into());
        let stored_fps = [(table.to_string(), stored_fp)].into();
        let current_rows = [(table.to_string(), target_rows)].into();
        let pk_map = [(table.to_string(), vec![pk_col("id")])].into();

        let result = svc.check(cs, &base, &stored_fps, &current_rows, &pk_map);
        assert!(result.is_clean(), "Different rows changed → no conflict");
    }
}
