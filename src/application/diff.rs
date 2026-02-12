use anyhow::Result;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::domain::table_diff::RowMap;
use crate::domain::{
    changeset::Changeset,
    ports::{Differ, RowRepository},
    table_diff::{ColumnDiff, RowChange, RowUpdate, TableDiff},
    value_objects::{ColumnName, Schema, TableName},
};
use crate::infrastructure::{config::TableConfig, db::sql_utils::pk_key};

// ─── Diff Service ───

pub struct DiffService {
    source_repo: Arc<dyn RowRepository>,
    target_repo: Arc<dyn RowRepository>,
    differ: Arc<dyn Differ>,
}

impl DiffService {
    pub fn new(
        source_repo: Arc<dyn RowRepository>,
        target_repo: Arc<dyn RowRepository>,
        differ: Arc<dyn Differ>,
    ) -> Self {
        Self {
            source_repo,
            target_repo,
            differ,
        }
    }

    pub async fn run_diff(
        &self,
        source_schema: &Schema,
        target_schema: &Schema,
        driver: &str,
        tables: &[TableConfig],
    ) -> Result<Changeset> {
        let mut handles = Vec::with_capacity(tables.len());

        for table_cfg in tables {
            let source_repo = Arc::clone(&self.source_repo);
            let target_repo = Arc::clone(&self.target_repo);
            let differ = Arc::clone(&self.differ);
            let source_schema = source_schema.clone();
            let target_schema = target_schema.clone();
            let table_cfg = table_cfg.clone();

            let handle = tokio::spawn(async move {
                let table_name = TableName(table_cfg.name.clone());
                let pk_cols: Vec<ColumnName> = table_cfg
                    .primary_key
                    .iter()
                    .map(|pk| ColumnName(pk.clone()))
                    .collect();

                let (source_rows, target_rows) = tokio::join!(
                    source_repo.fetch_rows(
                        &source_schema,
                        &table_name,
                        &pk_cols,
                        &table_cfg.excluded_columns
                    ),
                    target_repo.fetch_rows(
                        &target_schema,
                        &table_name,
                        &pk_cols,
                        &table_cfg.excluded_columns
                    )
                );

                let source_rows = source_rows?;
                let target_rows = target_rows?;

                Ok::<_, anyhow::Error>(differ.diff_table(
                    &source_rows,
                    &target_rows,
                    &pk_cols,
                    &table_name,
                ))
            });

            handles.push(handle);
        }

        // Collect results
        let mut table_diffs = Vec::with_capacity(handles.len());
        for h in handles {
            table_diffs.push(h.await??);
        }

        Ok(Changeset::new(
            &source_schema.0,
            &target_schema.0,
            driver,
            table_diffs,
        ))
    }
}

// ─── Table Differ (implementation of the port) ───

#[derive(Default)]
pub struct TableDiffer;

impl TableDiffer {
    pub fn new() -> Self {
        Self
    }
}

impl Differ for TableDiffer {
    fn diff_table(
        &self,
        source: &[RowMap],
        target: &[RowMap],
        pk_cols: &[ColumnName],
        table_name: &TableName,
    ) -> TableDiff {
        let source_index: BTreeMap<String, &RowMap> =
            source.iter().map(|r| (pk_key(r, pk_cols), r)).collect();
        let target_index: BTreeMap<String, &RowMap> =
            target.iter().map(|r| (pk_key(r, pk_cols), r)).collect();

        let source_keys: BTreeSet<&String> = source_index.keys().collect();
        let target_keys: BTreeSet<&String> = target_index.keys().collect();

        let insert_keys: Vec<&&String> = source_keys.difference(&target_keys).collect();
        let inserts: Vec<RowChange> = insert_keys
            .iter()
            .map(|k| {
                let row = source_index[k.as_str()];
                RowChange {
                    pk: extract_pk_from_row(row, pk_cols),
                    data: (*row).clone(),
                }
            })
            .collect();

        let delete_keys: Vec<&&String> = target_keys.difference(&source_keys).collect();
        let deletes: Vec<RowChange> = delete_keys
            .iter()
            .map(|k| {
                let row = target_index[k.as_str()];
                RowChange {
                    pk: extract_pk_from_row(row, pk_cols),
                    data: (*row).clone(),
                }
            })
            .collect();

        let common_keys: Vec<&&String> = source_keys.intersection(&target_keys).collect();
        let mut updates = Vec::new();

        for key in common_keys {
            let source_row = source_index[key.as_str()];
            let target_row = target_index[key.as_str()];

            let changed_columns = diff_columns(source_row, target_row);
            if !changed_columns.is_empty() {
                updates.push(RowUpdate {
                    pk: extract_pk_from_row(source_row, pk_cols),
                    before: (*target_row).clone(),
                    after: (*source_row).clone(),
                    changed_columns,
                });
            }
        }

        TableDiff {
            table_name: table_name.0.clone(),
            primary_key: pk_cols.iter().map(|c| c.0.clone()).collect(),
            inserts,
            updates,
            deletes,
        }
    }
}

// ─── Optimized diff logic ───

fn diff_columns(source: &RowMap, target: &RowMap) -> Vec<ColumnDiff> {
    let mut diffs = Vec::new();
    let all_keys: BTreeSet<_> = source.keys().chain(target.keys()).collect();

    for col in all_keys {
        let source_val = source.get(col).unwrap_or(&Value::Null);
        let target_val = target.get(col).unwrap_or(&Value::Null);

        // Fast path: hash equality
        if json_hash(source_val) == json_hash(target_val) {
            continue;
        }

        if !json_equal(source_val, target_val) {
            diffs.push(ColumnDiff {
                column: col.clone(),
                before: target_val.clone(),
                after: source_val.clone(),
            });
        }
    }

    diffs
}

fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => match (na.as_f64(), nb.as_f64()) {
            (Some(fa), Some(fb)) => float_eq(fa, fb),
            _ => na == nb,
        },
        _ => normalize_json(a) == normalize_json(b),
    }
}

fn float_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn json_hash(v: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_json(v, &mut hasher);
    hasher.finish()
}

fn hash_json(v: &Value, state: &mut impl Hasher) {
    match v {
        Value::Null => 0u8.hash(state),
        Value::Bool(b) => b.hash(state),
        Value::Number(n) => n.to_string().hash(state),
        Value::String(s) => s.hash(state),
        Value::Array(arr) => {
            for el in arr {
                hash_json(el, state);
            }
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            for (k, v) in entries {
                k.hash(state);
                hash_json(v, state);
            }
        }
    }
}

fn normalize_json(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k.clone(), normalize_json(v)))
                    .collect(),
            )
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json).collect()),
        _ => v.clone(),
    }
}

fn extract_pk_from_row(row: &RowMap, pk_cols: &[ColumnName]) -> BTreeMap<String, Value> {
    pk_cols
        .iter()
        .filter_map(|col| row.get(&col.0).map(|v| (col.0.clone(), v.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(pairs: &[(&str, Value)]) -> RowMap {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn col(name: &str) -> ColumnName {
        ColumnName(name.to_string())
    }

    fn table(name: &str) -> TableName {
        TableName(name.to_string())
    }

    // ── json_equal ──

    #[test]
    fn test_json_equal_identical_numbers() {
        assert!(json_equal(
            &Value::Number(1.into()),
            &Value::Number(1.into())
        ));
    }

    #[test]
    fn test_json_equal_float_within_epsilon() {
        let a = serde_json::Number::from_f64(0.1 + 0.2).unwrap();
        let b = serde_json::Number::from_f64(0.3).unwrap();
        assert!(json_equal(&Value::Number(a), &Value::Number(b)));
    }

    #[test]
    fn test_json_equal_strings_differ() {
        assert!(!json_equal(
            &Value::String("a".into()),
            &Value::String("b".into())
        ));
    }

    #[test]
    fn test_json_equal_ignores_object_key_order() {
        let a = json!({"a":1,"b":2});
        let b = json!({"b":2,"a":1});
        assert!(json_equal(&a, &b));
    }

    // ── extract_pk_from_row ──

    fn extract_pk(row: &RowMap, pk_cols: &[String]) -> BTreeMap<String, Value> {
        let pk_colnames: Vec<ColumnName> = pk_cols.iter().map(|s| ColumnName(s.clone())).collect();
        extract_pk_from_row(row, &pk_colnames)
    }

    #[test]
    fn test_extract_pk_single() {
        let r = row(&[
            ("id", Value::Number(42.into())),
            ("name", Value::String("x".into())),
        ]);
        let pk = extract_pk(&r, &["id".to_string()]);
        assert_eq!(pk.len(), 1);
        assert_eq!(pk["id"], Value::Number(42.into()));
    }

    #[test]
    fn test_extract_pk_composite() {
        let r = row(&[
            ("region_code", Value::String("FR".into())),
            ("product_category", Value::String("books".into())),
            (
                "tax_rate",
                Value::Number(serde_json::Number::from_f64(0.055).unwrap()),
            ),
        ]);
        let pk = extract_pk(
            &r,
            &["region_code".to_string(), "product_category".to_string()],
        );
        assert_eq!(pk.len(), 2);
        assert_eq!(pk["region_code"], Value::String("FR".into()));
        assert_eq!(pk["product_category"], Value::String("books".into()));
    }

    // ── diff_columns ──

    #[test]
    fn test_diff_columns_no_change() {
        let r = row(&[
            ("id", Value::Number(1.into())),
            ("val", Value::String("same".into())),
        ]);
        assert!(diff_columns(&r, &r).is_empty());
    }

    #[test]
    fn test_diff_columns_one_change() {
        let source = row(&[
            ("id", Value::Number(1.into())),
            ("val", Value::String("new".into())),
        ]);
        let target = row(&[
            ("id", Value::Number(1.into())),
            ("val", Value::String("old".into())),
        ]);
        let diffs = diff_columns(&source, &target);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].column, "val");
        assert_eq!(diffs[0].before, Value::String("old".into()));
        assert_eq!(diffs[0].after, Value::String("new".into()));
    }

    #[test]
    fn test_diff_columns_ignores_object_key_order() {
        let a = row(&[("meta", json!({"a":1,"b":2}))]);
        let b = row(&[("meta", json!({"b":2,"a":1}))]);
        assert!(diff_columns(&a, &b).is_empty());
    }

    #[test]
    fn test_diff_columns_float_tolerance() {
        let a = row(&[(
            "val",
            Value::Number(serde_json::Number::from_f64(1.0000000001).unwrap()),
        )]);
        let b = row(&[(
            "val",
            Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
        )]);
        assert!(diff_columns(&a, &b).is_empty());
    }

    // ── TableDiffer ──

    #[test]
    fn table_differ_detects_insert_update_delete() {
        let pk = vec![col("id")];
        let table = table("users");

        let source = vec![
            row(&[("id", json!(1)), ("name", json!("Alice"))]),
            row(&[("id", json!(2)), ("name", json!("Bob"))]),
        ];

        let target = vec![
            row(&[("id", json!(2)), ("name", json!("Bobby"))]),
            row(&[("id", json!(3)), ("name", json!("Charlie"))]),
        ];

        let differ = TableDiffer::new();
        let diff = differ.diff_table(&source, &target, &pk, &table);

        // insert: id=1
        assert_eq!(diff.inserts.len(), 1);
        assert_eq!(diff.inserts[0].pk["id"], json!(1));

        // delete: id=3
        assert_eq!(diff.deletes.len(), 1);
        assert_eq!(diff.deletes[0].pk["id"], json!(3));

        // update: id=2
        assert_eq!(diff.updates.len(), 1);
        let upd = &diff.updates[0];
        assert_eq!(upd.pk["id"], json!(2));
        assert_eq!(upd.changed_columns.len(), 1);
        assert_eq!(upd.changed_columns[0].column, "name");
    }

    #[test]
    fn no_diff_when_rows_identical() {
        let pk = vec![col("id")];
        let table = table("items");

        let rows = vec![
            row(&[("id", json!(1)), ("x", json!(10))]),
            row(&[("id", json!(2)), ("x", json!(20))]),
        ];

        let differ = TableDiffer::new();
        let diff = differ.diff_table(&rows, &rows, &pk, &table);

        assert!(diff.inserts.is_empty());
        assert!(diff.deletes.is_empty());
        assert!(diff.updates.is_empty());
    }

    #[test]
    fn test_diff_columns_nested_json() {
        let a = row(&[("json", json!({"a": 1, "b": [1,2,3], "c": {"x": 10}}))]);
        let b = row(&[("json", json!({"b": [1,2,3], "a": 1, "c": {"x": 10}}))]);
        assert!(diff_columns(&a, &b).is_empty());
    }
}
