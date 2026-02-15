use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;

use anyhow::Result;
use serde_json::Value;

use crate::domain::{changeset::Changeset, ports::OutputWriter, table_diff::ColumnDiff};
use crate::infrastructure::db::dialect::{from_driver, QueryDialect};

pub struct SqlWriter;

impl OutputWriter for SqlWriter {
    fn format(&self, changeset: &Changeset) -> Result<String> {
        let dialect = from_driver(&changeset.driver);
        let mut sql = String::new();

        writeln!(sql, "-- Changeset: {}", changeset.changeset_id)?;
        writeln!(sql, "-- Source: {}", changeset.source_schema)?;
        writeln!(sql, "-- Target: {}", changeset.target_schema)?;
        writeln!(sql, "-- Driver: {}", changeset.driver)?;
        writeln!(sql, "-- Generated: {}", changeset.created_at)?;
        writeln!(
            sql,
            "-- Summary: {} inserts, {} updates, {} deletes",
            changeset.summary.total_inserts,
            changeset.summary.total_updates,
            changeset.summary.total_deletes
        )?;
        writeln!(sql)?;
        writeln!(sql, "BEGIN;")?;
        writeln!(sql)?;

        for table in &changeset.tables {
            if table.is_empty() {
                continue;
            }

            writeln!(sql, "-- ============================================")?;
            writeln!(sql, "-- Table: {}", table.table_name)?;
            writeln!(sql, "-- ============================================")?;
            writeln!(sql)?;

            for del in &table.deletes {
                writeln!(
                    sql,
                    "DELETE FROM {}.{}",
                    dialect.quote_ident(&changeset.target_schema),
                    dialect.quote_ident(&table.table_name)
                )?;
                writeln!(
                    sql,
                    "  WHERE {};",
                    pk_where_clause(&del.pk, dialect.as_ref())
                )?;
                writeln!(sql)?;
            }

            for upd in &table.updates {
                writeln!(
                    sql,
                    "UPDATE {}.{}",
                    dialect.quote_ident(&changeset.target_schema),
                    dialect.quote_ident(&table.table_name)
                )?;
                writeln!(
                    sql,
                    "  SET {}",
                    set_clause(&upd.changed_columns, dialect.as_ref())
                )?;
                writeln!(
                    sql,
                    "  WHERE {};",
                    pk_where_clause(&upd.pk, dialect.as_ref())
                )?;
                writeln!(sql)?;
            }

            for ins in &table.inserts {
                let (cols, vals) = insert_columns_values(&ins.data, dialect.as_ref());
                writeln!(
                    sql,
                    "INSERT INTO {}.{} ({})",
                    dialect.quote_ident(&changeset.target_schema),
                    dialect.quote_ident(&table.table_name),
                    cols
                )?;
                writeln!(sql, "  VALUES ({});", vals)?;
                writeln!(sql)?;
            }
        }

        writeln!(sql, "COMMIT;")?;
        Ok(sql)
    }

    fn extension(&self) -> &'static str {
        "sql"
    }
}

pub(crate) fn pk_where_clause(pk: &BTreeMap<String, Value>, dialect: &dyn QueryDialect) -> String {
    pk.iter()
        .map(|(col, val)| {
            let col_q = dialect.quote_ident(col);
            if val == &Value::Null {
                format!("{} IS NULL", col_q)
            } else {
                format!("{} = {}", col_q, dialect.sql_literal(val))
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub(crate) fn set_clause(columns: &[ColumnDiff], dialect: &dyn QueryDialect) -> String {
    columns
        .iter()
        .map(|c| {
            format!(
                "{} = {}",
                dialect.quote_ident(&c.column),
                dialect.sql_literal(&c.after)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn insert_columns_values(
    data: &BTreeMap<String, Value>,
    dialect: &dyn QueryDialect,
) -> (String, String) {
    let cols: Vec<String> = data.keys().map(|k| dialect.quote_ident(k)).collect();
    let vals: Vec<String> = data.values().map(|v| dialect.sql_literal(v)).collect();
    (cols.join(", "), vals.join(", "))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — use dialect instances directly, same assertions as before
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::dialect::{MysqlDialect, PostgresDialect, SqliteDialect};
    use serde_json::json;

    fn pg() -> PostgresDialect {
        PostgresDialect
    }
    fn my() -> MysqlDialect {
        MysqlDialect
    }
    fn sq() -> SqliteDialect {
        SqliteDialect
    }

    #[test]
    fn test_pk_where_clause_null_is_null() {
        let mut pk = BTreeMap::new();
        pk.insert("id".to_string(), Value::Null);
        assert_eq!(pk_where_clause(&pk, &pg()), r#""id" IS NULL"#);
    }

    #[test]
    fn test_pk_where_clause_value() {
        let mut pk = BTreeMap::new();
        pk.insert("id".to_string(), json!(42));
        assert_eq!(pk_where_clause(&pk, &pg()), r#""id" = 42"#);
    }

    #[test]
    fn test_pk_where_clause_mysql_backticks() {
        let mut pk = BTreeMap::new();
        pk.insert("id".to_string(), json!(1));
        assert_eq!(pk_where_clause(&pk, &my()), "`id` = 1");
    }

    #[test]
    fn test_sql_literal_null() {
        assert_eq!(pg().sql_literal(&Value::Null), "NULL");
        assert_eq!(my().sql_literal(&Value::Null), "NULL");
    }

    #[test]
    fn test_sql_literal_bool() {
        assert_eq!(pg().sql_literal(&Value::Bool(true)), "TRUE");
        assert_eq!(my().sql_literal(&Value::Bool(false)), "FALSE");
    }

    #[test]
    fn test_sql_literal_number() {
        assert_eq!(pg().sql_literal(&json!(19.99)), "19.99");
        assert_eq!(my().sql_literal(&json!(42)), "42");
    }

    #[test]
    fn test_sql_literal_string_escapes_quotes() {
        let v = Value::String("it's fine".to_string());
        assert_eq!(pg().sql_literal(&v), "'it''s fine'");
    }

    #[test]
    fn test_sql_literal_jsonb_postgres() {
        let v = json!({"key": "val"});
        let lit = pg().sql_literal(&v);
        assert!(
            lit.ends_with("::jsonb"),
            "Expected ::jsonb cast, got: {}",
            lit
        );
        assert!(lit.starts_with('\''));
    }

    #[test]
    fn test_sql_literal_json_mysql_no_cast() {
        let v = json!({"key": "val"});
        let lit = my().sql_literal(&v);
        assert!(
            !lit.contains("::jsonb"),
            "MySQL must not have ::jsonb, got: {}",
            lit
        );
        assert!(lit.starts_with('\''));
    }

    #[test]
    fn test_sql_literal_json_sqlite_no_cast() {
        let v = json!([1, 2, 3]);
        let lit = sq().sql_literal(&v);
        assert!(
            !lit.contains("::"),
            "SQLite must not have any cast, got: {}",
            lit
        );
        assert!(lit.starts_with('\''));
    }
}
