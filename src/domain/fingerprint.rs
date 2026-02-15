use sha2::{Digest, Sha256};

use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::Fingerprint;

/// Compute a SHA-256 fingerprint of a table's row content.
///
/// Algorithm:
/// 1. Each row is serialised to a **canonical** JSON string (keys sorted by
///    `BTreeMap` — already guaranteed by `RowMap`).
/// 2. Rows are sorted lexicographically by their JSON representation so the
///    fingerprint is stable regardless of the order rows are returned by the DB.
/// 3. All row strings are joined with `\n` and hashed with SHA-256.
///
/// An empty table produces a well-defined fingerprint (hash of empty string).
/// ```
pub fn fingerprint(rows: &[RowMap]) -> Fingerprint {
    let mut row_strings: Vec<String> = rows
        .iter()
        .map(|row| serde_json::to_string(row).unwrap_or_default())
        .collect();

    // Sort rows for stability — DB order is not guaranteed to be consistent.
    row_strings.sort_unstable();

    let content = row_strings.join("\n");
    let hash = Sha256::digest(content.as_bytes());
    Fingerprint(format!("{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(pairs: &[(&str, serde_json::Value)]) -> RowMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn same_rows_same_fingerprint() {
        let rows = vec![
            row(&[("id", json!(1)), ("val", json!("a"))]),
            row(&[("id", json!(2)), ("val", json!("b"))]),
        ];
        assert_eq!(fingerprint(&rows), fingerprint(&rows));
    }

    #[test]
    fn different_rows_different_fingerprint() {
        let rows_a = vec![row(&[("id", json!(1)), ("val", json!("a"))])];
        let rows_b = vec![row(&[("id", json!(1)), ("val", json!("CHANGED"))])];
        assert_ne!(fingerprint(&rows_a), fingerprint(&rows_b));
    }

    #[test]
    fn order_independent() {
        let row1 = row(&[("id", json!(1)), ("val", json!("a"))]);
        let row2 = row(&[("id", json!(2)), ("val", json!("b"))]);
        assert_eq!(
            fingerprint(&[row1.clone(), row2.clone()]),
            fingerprint(&[row2, row1]),
        );
    }

    #[test]
    fn empty_table_is_deterministic() {
        assert_eq!(fingerprint(&[]), fingerprint(&[]));
    }
}
