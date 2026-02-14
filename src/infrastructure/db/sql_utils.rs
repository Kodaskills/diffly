use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::{ColumnName, Schema, TableName};
use crate::infrastructure::db::dialect::QueryDialect;
use serde_json::Value;

// ─────────────────────────────────────────────────────────────────────────────
// Query builders
// ─────────────────────────────────────────────────────────────────────────────

/// Build a `SELECT * FROM <schema>.<table> ORDER BY <pk_cols>` query.
/// Used for SQLite (no introspection needed) and as the fallback path.
/// `ORDER BY` is omitted when `pk_cols` is empty to avoid a SQL syntax error.
pub fn build_select_query(
    schema: &Schema,
    table: &TableName,
    pk_cols: &[ColumnName],
    dialect: &dyn QueryDialect,
) -> String {
    let prefix = dialect.schema_prefix(&schema.0);
    let table_q = dialect.quote_ident(&table.0);
    let order_cols: Vec<String> = pk_cols.iter().map(|c| dialect.quote_ident(&c.0)).collect();
    if order_cols.is_empty() {
        format!("SELECT * FROM {}{}", prefix, table_q)
    } else {
        format!(
            "SELECT * FROM {}{} ORDER BY {}",
            prefix,
            table_q,
            order_cols.join(", ")
        )
    }
}

/// Build a typed SELECT where every column whose `information_schema.data_type`
/// is not natively supported by `sqlx::AnyRow` is wrapped in the dialect cast
/// expression (e.g. `::TEXT` for PostgreSQL, `CONVERT(… USING utf8mb4)` for MySQL).
///
/// `col_types` is a vec of `(column_name, data_type)` pairs in ordinal order,
/// obtained from `information_schema.columns`.
pub fn build_typed_select_query(
    schema: &Schema,
    table: &TableName,
    pk_cols: &[ColumnName],
    col_types: &[(String, String)],
    dialect: &dyn QueryDialect,
) -> String {
    let prefix = dialect.schema_prefix(&schema.0);
    let table_q = dialect.quote_ident(&table.0);

    let col_exprs: Vec<String> = col_types
        .iter()
        .map(|(col_name, data_type)| {
            let q = dialect.quote_ident(col_name);
            if dialect.is_native_type(data_type) {
                q
            } else {
                dialect.cast_to_text(&q)
            }
        })
        .collect();

    let order_cols: Vec<String> = pk_cols.iter().map(|c| dialect.quote_ident(&c.0)).collect();

    if order_cols.is_empty() {
        format!("SELECT {} FROM {}{}", col_exprs.join(", "), prefix, table_q)
    } else {
        format!(
            "SELECT {} FROM {}{} ORDER BY {}",
            col_exprs.join(", "),
            prefix,
            table_q,
            order_cols.join(", ")
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Row helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build the composite primary key string for a row (used as BTreeMap lookup key).
pub fn pk_key(row: &RowMap, pk_cols: &[ColumnName]) -> String {
    pk_cols
        .iter()
        .map(|col| {
            row.get(&col.0)
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| "NULL".to_string())
        })
        .collect::<Vec<_>>()
        .join("|")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{ColumnName, Schema, TableName};
    use crate::infrastructure::db::dialect::{MysqlDialect, PostgresDialect, SqliteDialect};

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
    fn test_build_select_query_postgres() {
        let schema = Schema("sandbox".into());
        let table = TableName("pricing_rules".into());
        let pks = vec![ColumnName("id".into())];
        let q = build_select_query(&schema, &table, &pks, &pg());
        assert_eq!(
            q,
            r#"SELECT * FROM "sandbox"."pricing_rules" ORDER BY "id""#
        );
    }

    #[test]
    fn test_build_select_query_mysql() {
        let schema = Schema("mydb".into());
        let table = TableName("rules".into());
        let pks = vec![ColumnName("id".into()), ColumnName("code".into())];
        let q = build_select_query(&schema, &table, &pks, &my());
        assert_eq!(q, "SELECT * FROM `mydb`.`rules` ORDER BY `id`, `code`");
    }

    #[test]
    fn test_build_select_query_sqlite() {
        let schema = Schema("ignored".into());
        let table = TableName("rules".into());
        let pks = vec![ColumnName("id".into())];
        let q = build_select_query(&schema, &table, &pks, &sq());
        assert_eq!(q, r#"SELECT * FROM "rules" ORDER BY "id""#);
    }

    #[test]
    fn test_build_select_query_no_pk_omits_order_by() {
        let schema = Schema("s".into());
        let table = TableName("t".into());
        let pks: Vec<ColumnName> = vec![];
        let q = build_select_query(&schema, &table, &pks, &pg());
        assert_eq!(q, r#"SELECT * FROM "s"."t""#);
        assert!(!q.contains("ORDER BY"));
    }

    #[test]
    fn test_build_typed_select_query_postgres_casts_non_primitives() {
        let schema = Schema("sandbox".into());
        let table = TableName("pricing_rules".into());
        let pks = vec![ColumnName("id".into())];
        let col_types = vec![
            ("id".to_string(), "integer".to_string()),
            ("name".to_string(), "character varying".to_string()),
            ("price".to_string(), "numeric".to_string()),
            ("uid".to_string(), "uuid".to_string()),
            ("active".to_string(), "boolean".to_string()),
        ];
        let q = build_typed_select_query(&schema, &table, &pks, &col_types, &pg());
        assert!(!q.contains(r#""id"::TEXT"#));
        assert!(!q.contains(r#""active"::TEXT"#));
        assert!(q.contains(r#""name"::TEXT"#));
        assert!(q.contains(r#""price"::TEXT"#));
        assert!(q.contains(r#""uid"::TEXT"#));
        assert!(q.contains(r#"ORDER BY "id""#));
    }

    #[test]
    fn test_build_typed_select_query_mysql_uses_convert() {
        let schema = Schema("source_db".into());
        let table = TableName("pricing_rules".into());
        let pks = vec![ColumnName("id".into())];
        let col_types = vec![
            ("id".to_string(), "int".to_string()),
            ("discount_rate".to_string(), "decimal".to_string()),
            ("is_active".to_string(), "tinyint".to_string()),
            ("metadata".to_string(), "json".to_string()),
        ];
        let q = build_typed_select_query(&schema, &table, &pks, &col_types, &my());
        assert!(!q.contains("CONVERT(`id`"), "int should not be cast");
        assert!(
            q.contains("CONVERT(`is_active` USING utf8mb4)"),
            "tinyint should now be cast"
        );
        assert!(
            q.contains("CONVERT(`discount_rate` USING utf8mb4)"),
            "{}",
            q
        );
        assert!(q.contains("CONVERT(`metadata` USING utf8mb4)"), "{}", q);
        assert!(!q.contains("::TEXT"), "{}", q);
        assert!(q.contains("ORDER BY `id`"));
    }

    #[test]
    fn test_build_typed_select_query_array_gets_text_cast() {
        let schema = Schema("s".into());
        let table = TableName("t".into());
        let pks = vec![ColumnName("id".into())];
        let col_types = vec![
            ("id".to_string(), "integer".to_string()),
            ("tags".to_string(), "ARRAY".to_string()),
        ];
        let q = build_typed_select_query(&schema, &table, &pks, &col_types, &pg());
        assert!(q.contains(r#""tags"::TEXT"#), "{}", q);
        assert!(!q.contains(r#""id"::TEXT"#), "{}", q);
    }

    #[test]
    fn test_build_typed_select_query_no_pk_omits_order_by() {
        let schema = Schema("s".into());
        let table = TableName("t".into());
        let pks: Vec<ColumnName> = vec![];
        let col_types = vec![("id".to_string(), "integer".to_string())];
        let q = build_typed_select_query(&schema, &table, &pks, &col_types, &pg());
        assert!(!q.contains("ORDER BY"));
    }
}
