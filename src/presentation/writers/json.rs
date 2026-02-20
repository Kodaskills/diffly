use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;

use crate::application::monitoring::PerfReport;
use crate::domain::{
    changeset::{Changeset, Summary},
    ports::OutputWriter,
    table_diff::{ColumnDiff, RowChange, RowUpdate, TableDiff},
};
use crate::infrastructure::db::dialect::{from_driver, QueryDialect};
use crate::presentation::writers::sql::{insert_columns_values, pk_where_clause, set_clause};

// ─── Serialisation view types ─────────────────────────────────────────────────
//
// These mirror the domain structs but add a `sql` field to each change entry.
// They are presentation-only: the domain types are never modified.

#[derive(Serialize)]
struct JsonChangeset<'a> {
    changeset_id: &'a str,
    source_schema: &'a str,
    target_schema: &'a str,
    driver: &'a str,
    created_at: &'a str,
    source_fingerprint: &'a str,
    target_fingerprint: &'a str,
    tables: Vec<JsonTableDiff<'a>>,
    summary: &'a Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    perf: Option<&'a PerfReport>,
}

#[derive(Serialize)]
struct JsonTableDiff<'a> {
    table_name: &'a str,
    primary_key: &'a [String],
    inserts: Vec<JsonInsert<'a>>,
    updates: Vec<JsonUpdate<'a>>,
    deletes: Vec<JsonDelete<'a>>,
}

#[derive(Serialize)]
struct JsonInsert<'a> {
    pk: &'a BTreeMap<String, Value>,
    data: &'a BTreeMap<String, Value>,
    sql: String,
}

#[derive(Serialize)]
struct JsonUpdate<'a> {
    pk: &'a BTreeMap<String, Value>,
    before: &'a BTreeMap<String, Value>,
    after: &'a BTreeMap<String, Value>,
    changed_columns: &'a [ColumnDiff],
    sql: String,
}

#[derive(Serialize)]
struct JsonDelete<'a> {
    pk: &'a BTreeMap<String, Value>,
    data: &'a BTreeMap<String, Value>,
    sql: String,
}

// ─── SQL generation helpers ───────────────────────────────────────────────────

fn insert_sql(schema: &str, table: &str, row: &RowChange, dialect: &dyn QueryDialect) -> String {
    let (cols, vals) = insert_columns_values(&row.data, dialect);
    let mut s = String::new();
    let _ = write!(
        s,
        "INSERT INTO {}.{} ({}) VALUES ({});",
        dialect.quote_ident(schema),
        dialect.quote_ident(table),
        cols,
        vals
    );
    s
}

fn update_sql(schema: &str, table: &str, row: &RowUpdate, dialect: &dyn QueryDialect) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "UPDATE {}.{} SET {} WHERE {};",
        dialect.quote_ident(schema),
        dialect.quote_ident(table),
        set_clause(&row.changed_columns, dialect),
        pk_where_clause(&row.pk, dialect),
    );
    s
}

fn delete_sql(schema: &str, table: &str, row: &RowChange, dialect: &dyn QueryDialect) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "DELETE FROM {}.{} WHERE {};",
        dialect.quote_ident(schema),
        dialect.quote_ident(table),
        pk_where_clause(&row.pk, dialect),
    );
    s
}

// ─── View builder ─────────────────────────────────────────────────────────────

fn build_table_diff<'a>(
    table: &'a TableDiff,
    schema: &str,
    dialect: &dyn QueryDialect,
) -> JsonTableDiff<'a> {
    JsonTableDiff {
        table_name: &table.table_name,
        primary_key: &table.primary_key,
        inserts: table
            .inserts
            .iter()
            .map(|r| JsonInsert {
                pk: &r.pk,
                data: &r.data,
                sql: insert_sql(schema, &table.table_name, r, dialect),
            })
            .collect(),
        updates: table
            .updates
            .iter()
            .map(|r| JsonUpdate {
                pk: &r.pk,
                before: &r.before,
                after: &r.after,
                changed_columns: &r.changed_columns,
                sql: update_sql(schema, &table.table_name, r, dialect),
            })
            .collect(),
        deletes: table
            .deletes
            .iter()
            .map(|r| JsonDelete {
                pk: &r.pk,
                data: &r.data,
                sql: delete_sql(schema, &table.table_name, r, dialect),
            })
            .collect(),
    }
}

// ─── Writer ───────────────────────────────────────────────────────────────────

pub struct JsonWriter;

impl OutputWriter for JsonWriter {
    fn format(&self, cs: &Changeset) -> Result<String> {
        let dialect = from_driver(&cs.driver);

        let view = JsonChangeset {
            changeset_id: &cs.changeset_id,
            source_schema: &cs.source_schema,
            target_schema: &cs.target_schema,
            driver: &cs.driver,
            created_at: &cs.created_at,
            source_fingerprint: &cs.source_fingerprint,
            target_fingerprint: &cs.target_fingerprint,
            tables: cs
                .tables
                .iter()
                .map(|t| build_table_diff(t, &cs.target_schema, dialect.as_ref()))
                .collect(),
            summary: &cs.summary,
            perf: cs.perf.as_ref(),
        };

        Ok(serde_json::to_string_pretty(&view)?)
    }

    fn extension(&self) -> &'static str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::changeset::Changeset;
    use crate::domain::table_diff::{ColumnDiff, RowChange, RowUpdate, TableDiff};
    use serde_json::{json, Value};

    fn make_changeset() -> Changeset {
        let insert = RowChange {
            pk: [("id".to_string(), json!(1))].into(),
            data: [
                ("id".to_string(), json!(1)),
                ("rate".to_string(), json!(0.10)),
            ]
            .into(),
        };
        let update = RowUpdate {
            pk: [("id".to_string(), json!(2))].into(),
            before: [("rate".to_string(), json!(0.20))].into(),
            after: [("rate".to_string(), json!(0.25))].into(),
            changed_columns: vec![ColumnDiff {
                column: "rate".to_string(),
                before: json!(0.20),
                after: json!(0.25),
            }],
        };
        let delete = RowChange {
            pk: [("id".to_string(), json!(3))].into(),
            data: [
                ("id".to_string(), json!(3)),
                ("rate".to_string(), json!(0.30)),
            ]
            .into(),
        };

        let table = TableDiff {
            table_name: "pricing_rules".to_string(),
            primary_key: vec!["id".to_string()],
            inserts: vec![insert],
            updates: vec![update],
            deletes: vec![delete],
        };

        Changeset::new("public", "public", "postgres", vec![table])
    }

    #[test]
    fn json_output_contains_sql_field_for_each_change() {
        let cs = make_changeset();
        let output = JsonWriter.format(&cs).unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let table = &parsed["tables"][0];

        let insert_sql = table["inserts"][0]["sql"].as_str().unwrap();
        assert!(insert_sql.starts_with("INSERT INTO"), "got: {insert_sql}");
        assert!(insert_sql.contains("pricing_rules"), "got: {insert_sql}");

        let update_sql = table["updates"][0]["sql"].as_str().unwrap();
        assert!(update_sql.starts_with("UPDATE"), "got: {update_sql}");
        assert!(update_sql.contains("rate"), "got: {update_sql}");
        assert!(update_sql.contains("WHERE"), "got: {update_sql}");

        let delete_sql = table["deletes"][0]["sql"].as_str().unwrap();
        assert!(delete_sql.starts_with("DELETE FROM"), "got: {delete_sql}");
        assert!(delete_sql.contains("WHERE"), "got: {delete_sql}");
    }

    #[test]
    fn json_output_sql_uses_correct_dialect_quoting() {
        let mut cs = make_changeset();
        cs.driver = "mysql".to_string();
        let output = JsonWriter.format(&cs).unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let insert_sql = parsed["tables"][0]["inserts"][0]["sql"].as_str().unwrap();
        // MySQL uses backticks
        assert!(
            insert_sql.contains('`'),
            "expected backticks, got: {insert_sql}"
        );
    }
}
