use anyhow::Result;
use serde_json::{json, Value};
use sqlx::any::AnyRow;
use sqlx::{Column, Row, TypeInfo};

// ─────────────────────────────────────────────────────────────────────────────
// Traits
// ─────────────────────────────────────────────────────────────────────────────

/// SQL dialect: query building and literal formatting.
///
/// Implemented per driver. Used by both the infrastructure query builders
/// and the presentation SQL writer — the interface is pure string manipulation
/// with no sqlx dependency, so it crosses the layer boundary cleanly.
pub trait QueryDialect: Send + Sync {
    /// Return the driver name as a lowercase string ("postgres", "mysql", …).
    /// Used for output metadata (SQL file header, HTML report) only —
    /// never for branching logic (use the other methods for that).
    fn name(&self) -> &'static str;

    /// Return `true` if this dialect supports `information_schema.columns`
    /// introspection, enabling the typed SELECT path.
    /// Defaults to `true`; override to `false` for SQLite (no information_schema).
    fn needs_introspection(&self) -> bool {
        true
    }

    /// Quote an identifier (table, column, schema) per dialect.
    /// - MySQL / MariaDB → backtick: `` `col` ``
    /// - PostgreSQL / SQLite → double-quote: `"col"`
    fn quote_ident(&self, s: &str) -> String;

    /// Return the `schema.` prefix for a qualified table reference.
    /// SQLite has no schema namespace, so it returns `""`.
    fn schema_prefix(&self, schema: &str) -> String {
        format!("{}.", self.quote_ident(schema))
    }

    /// Produce the cast expression that coerces an unsupported column type to
    /// a string readable by `sqlx::AnyRow`.
    /// - PostgreSQL  : `"col"::TEXT AS "col"`
    /// - MySQL/MariaDB : `CONVERT(\`col\` USING utf8mb4) AS \`col\``
    fn cast_to_text(&self, col_quoted: &str) -> String;

    /// Return `true` if `data_type` (an `information_schema.data_type` value)
    /// is natively decodable by `sqlx::AnyRow` without any explicit cast.
    fn is_native_type(&self, data_type: &str) -> bool;

    /// The SQL to introspect column types from `information_schema.columns`.
    /// Uses driver-appropriate placeholders ($1/$2 vs ?/?)
    /// and driver-appropriate casts (::TEXT vs nothing).
    fn introspect_sql(&self) -> &'static str;

    /// Format a JSON `Value` as an SQL literal for this dialect.
    /// - NULL          → `NULL`
    /// - Bool          → `TRUE` / `FALSE`
    /// - Number        → bare number
    /// - String        → `'escaped'`
    /// - Object/Array  → `'json'` with `::jsonb` cast on PostgreSQL only
    fn sql_literal(&self, val: &Value) -> String {
        match val {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => format!("'{}'", s.replace('\'', "''")),
            Value::Array(_) | Value::Object(_) => {
                let json_str = serde_json::to_string(val)
                    .unwrap_or_default()
                    .replace('\'', "''");
                self.json_literal(&json_str)
            }
        }
    }

    /// Render a pre-serialised JSON string as a dialect-appropriate literal.
    /// Override in PostgreSQL to append `::jsonb`.
    fn json_literal(&self, json_str: &str) -> String {
        format!("'{}'", json_str)
    }
}

/// Row decoder: read a single `AnyRow` column into a `serde_json::Value`.
///
/// Implemented per driver. Lives in infrastructure only — callers outside
/// this module receive `Value`s, never raw `AnyRow`s.
pub trait RowDecoder: Send + Sync {
    /// Decode the column at `idx` using `type_hint` (an `information_schema`
    /// `data_type` string) to reconstruct the correct `Value` variant.
    fn decode_column(&self, row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value>;
}

// ─────────────────────────────────────────────────────────────────────────────
// PostgreSQL
// ─────────────────────────────────────────────────────────────────────────────

pub struct PostgresDialect;

impl QueryDialect for PostgresDialect {
    fn name(&self) -> &'static str {
        "postgres"
    }

    fn quote_ident(&self, s: &str) -> String {
        format!("\"{}\"", s.replace('"', "\"\""))
    }

    fn cast_to_text(&self, col_quoted: &str) -> String {
        format!("{}::TEXT AS {}", col_quoted, col_quoted)
    }

    fn is_native_type(&self, data_type: &str) -> bool {
        matches!(
            data_type.to_lowercase().as_str(),
            "boolean" | "smallint" | "integer" | "bigint" | "real" | "double precision"
        )
    }

    fn introspect_sql(&self) -> &'static str {
        "SELECT column_name::TEXT, data_type::TEXT \
         FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 \
         ORDER BY ordinal_position"
    }

    fn json_literal(&self, json_str: &str) -> String {
        format!("'{}'::jsonb", json_str)
    }
}

impl RowDecoder for PostgresDialect {
    fn decode_column(&self, row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value> {
        col_to_json(row, idx, type_hint)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MySQL / MariaDB
// ─────────────────────────────────────────────────────────────────────────────

pub struct MysqlDialect;

impl QueryDialect for MysqlDialect {
    fn name(&self) -> &'static str {
        "mysql"
    }

    fn quote_ident(&self, s: &str) -> String {
        format!("`{}`", s.replace('`', "``"))
    }

    fn cast_to_text(&self, col_quoted: &str) -> String {
        // CAST(col AS CHAR) and CONVERT(col USING utf8mb4) both return BLOB
        // to sqlx AnyRow — we detect BLOB in the mapper and read Vec<u8>.
        format!("CONVERT({} USING utf8mb4) AS {}", col_quoted, col_quoted)
    }

    fn is_native_type(&self, data_type: &str) -> bool {
        matches!(
            data_type.to_lowercase().as_str(),
            "int" | "mediumint" | "bigint" | "float" | "double"
        )
    }

    fn introspect_sql(&self) -> &'static str {
        "SELECT column_name, data_type \
         FROM information_schema.columns \
         WHERE table_schema = ? AND table_name = ? \
         ORDER BY ordinal_position"
    }
    // json_literal: default (no ::jsonb cast)
}

impl RowDecoder for MysqlDialect {
    fn decode_column(&self, row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value> {
        // MySQL returns non-native columns as BLOB regardless of any SQL cast.
        // Detect at runtime and read raw bytes, then reinterpret using the type hint.
        let anyrow_type = row.column(idx).type_info().name();
        if anyrow_type == "BLOB" {
            blob_to_json(row, idx, type_hint)
        } else {
            col_to_json(row, idx, type_hint)
        }
    }
}

// MariaDB shares MySQL's wire protocol and AnyRow behaviour.
pub struct MariadbDialect;

impl QueryDialect for MariadbDialect {
    fn name(&self) -> &'static str {
        "mariadb"
    }

    fn quote_ident(&self, s: &str) -> String {
        MysqlDialect.quote_ident(s)
    }

    fn cast_to_text(&self, col_quoted: &str) -> String {
        MysqlDialect.cast_to_text(col_quoted)
    }

    fn is_native_type(&self, data_type: &str) -> bool {
        MysqlDialect.is_native_type(data_type)
    }

    fn introspect_sql(&self) -> &'static str {
        MysqlDialect.introspect_sql()
    }
}

impl RowDecoder for MariadbDialect {
    fn decode_column(&self, row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value> {
        MysqlDialect.decode_column(row, idx, type_hint)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SQLite
// ─────────────────────────────────────────────────────────────────────────────

pub struct SqliteDialect;

impl QueryDialect for SqliteDialect {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn needs_introspection(&self) -> bool {
        false
    }

    fn quote_ident(&self, s: &str) -> String {
        format!("\"{}\"", s.replace('"', "\"\""))
    }

    fn schema_prefix(&self, _schema: &str) -> String {
        // SQLite has no schema namespace
        String::new()
    }

    fn cast_to_text(&self, col_quoted: &str) -> String {
        format!("CAST({} AS TEXT) AS {}", col_quoted, col_quoted)
    }

    fn is_native_type(&self, data_type: &str) -> bool {
        // SQLite uses type affinity — all common storage classes are native.
        matches!(
            data_type.to_uppercase().as_str(),
            "INTEGER" | "INT" | "REAL" | "NUMERIC" | "TEXT" | "BLOB"
        )
    }

    fn introspect_sql(&self) -> &'static str {
        // SQLite does not have information_schema; this path is not used.
        // fetch_column_types is only called for postgres/mysql/mariadb.
        ""
    }
    // json_literal: default (no ::jsonb cast)
}

impl RowDecoder for SqliteDialect {
    fn decode_column(&self, row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value> {
        col_to_json(row, idx, type_hint)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the dialect pair (QueryDialect + RowDecoder) from a driver name string.
/// Returns `Box<dyn Dialect>` where `Dialect` is the combined supertrait alias.
pub fn from_driver(driver: &str) -> Box<dyn Dialect> {
    match driver {
        "mysql" => Box::new(MysqlDialect),
        "mariadb" => Box::new(MariadbDialect),
        "sqlite" => Box::new(SqliteDialect),
        _ => Box::new(PostgresDialect),
    }
}

/// Combined supertrait — convenience alias so callers only store one object.
pub trait Dialect: QueryDialect + RowDecoder {}
impl Dialect for PostgresDialect {}
impl Dialect for MysqlDialect {}
impl Dialect for MariadbDialect {}
impl Dialect for SqliteDialect {}

// ─────────────────────────────────────────────────────────────────────────────
// Shared decoding helpers (private to this module)
// ─────────────────────────────────────────────────────────────────────────────

/// Decode a BLOB column (MySQL/MariaDB non-native types) as raw UTF-8 bytes,
/// then reinterpret the string using the `information_schema` type hint.
fn blob_to_json(row: &AnyRow, idx: usize, type_hint: &str) -> Result<Value> {
    let bytes: Option<Vec<u8>> = row.try_get(idx)?;
    let Some(b) = bytes else {
        return Ok(Value::Null);
    };
    let s = String::from_utf8(b).unwrap_or_default();
    Ok(match type_hint.to_uppercase().as_str() {
        "DECIMAL" | "NUMERIC" => s
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(s)),
        "JSON" | "JSONB" => serde_json::from_str(&s).unwrap_or(Value::String(s)),
        _ => Value::String(s),
    })
}

/// Decode a column whose AnyRow type is supported natively or has been
/// cast to TEXT in the SELECT query.
fn col_to_json(row: &AnyRow, idx: usize, type_name: &str) -> Result<Value> {
    let v = match type_name.to_uppercase().as_str() {
        // ── Booleans ──────────────────────────────────────────────────────────
        "BOOL" | "BOOLEAN" => row
            .try_get::<Option<bool>, _>(idx)?
            .map_or(Value::Null, Value::Bool),

        // ── Integers ──────────────────────────────────────────────────────────
        "INT2" | "SMALLINT" | "SMALLSERIAL" => row
            .try_get::<Option<i32>, _>(idx)?
            .map_or(Value::Null, |v| json!(v)),

        "TINYINT" => match row.try_get::<Option<String>, _>(idx)? {
            None => Value::Null,
            Some(s) => s
                .parse::<i32>()
                .map(|v| json!(v))
                .unwrap_or_else(|_| Value::String(s)),
        },

        "INT4" | "INT" | "INTEGER" | "SERIAL" => row
            .try_get::<Option<i32>, _>(idx)?
            .map_or(Value::Null, |v| json!(v)),

        "INT8" | "BIGINT" | "BIGSERIAL" => row
            .try_get::<Option<i64>, _>(idx)?
            .map_or(Value::Null, |v| json!(v)),

        // ── Floats ────────────────────────────────────────────────────────────
        "FLOAT4" | "REAL" | "FLOAT" => row
            .try_get::<Option<f32>, _>(idx)?
            .map_or(Value::Null, |v| json!(v as f64)),

        "FLOAT8" | "DOUBLE" | "DOUBLE PRECISION" => row
            .try_get::<Option<f64>, _>(idx)?
            .map_or(Value::Null, |v| json!(v)),

        // ── NUMERIC / DECIMAL → cast to TEXT in SELECT, parse back to Number ─
        "NUMERIC" | "DECIMAL" => match row.try_get::<Option<String>, _>(idx)? {
            None => Value::Null,
            Some(s) => s
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::String(s)),
        },

        // ── JSON / JSONB → cast to TEXT in SELECT, parse back to Value ────────
        "JSON" | "JSONB" => match row.try_get::<Option<String>, _>(idx)? {
            None => Value::Null,
            Some(s) => serde_json::from_str(&s).unwrap_or(Value::String(s)),
        },

        // ── ARRAY (PostgreSQL) → stored as Value::String ──────────────────────
        "ARRAY" => row
            .try_get::<Option<String>, _>(idx)?
            .map_or(Value::Null, Value::String),

        // ── Everything else: TEXT, VARCHAR, CHAR, UUID, TIMESTAMP, DATE …
        _ => row
            .try_get::<Option<String>, _>(idx)?
            .map_or(Value::Null, Value::String),
    };
    Ok(v)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── QueryDialect — quote_ident ──────────────────────────────────────────

    #[test]
    fn test_postgres_quote_ident() {
        let d = PostgresDialect;
        assert_eq!(d.quote_ident("my_table"), r#""my_table""#);
        assert_eq!(d.quote_ident(r#"ta"ble"#), r#""ta""ble""#);
    }

    #[test]
    fn test_mysql_quote_ident() {
        let d = MysqlDialect;
        assert_eq!(d.quote_ident("my_table"), "`my_table`");
        assert_eq!(d.quote_ident("ta`ble"), "`ta``ble`");
    }

    #[test]
    fn test_sqlite_quote_ident() {
        let d = SqliteDialect;
        assert_eq!(d.quote_ident("my_table"), r#""my_table""#);
    }

    // ── QueryDialect — schema_prefix ───────────────────────────────────────

    #[test]
    fn test_postgres_schema_prefix() {
        assert_eq!(PostgresDialect.schema_prefix("sandbox"), r#""sandbox"."#);
    }

    #[test]
    fn test_sqlite_schema_prefix_empty() {
        assert_eq!(SqliteDialect.schema_prefix("ignored"), "");
    }

    #[test]
    fn test_mysql_schema_prefix() {
        assert_eq!(MysqlDialect.schema_prefix("mydb"), "`mydb`.");
    }

    // ── QueryDialect — cast_to_text ─────────────────────────────────────────

    #[test]
    fn test_postgres_cast_to_text() {
        assert_eq!(
            PostgresDialect.cast_to_text(r#""price""#),
            r#""price"::TEXT AS "price""#
        );
    }

    #[test]
    fn test_mysql_cast_to_text() {
        assert_eq!(
            MysqlDialect.cast_to_text("`price`"),
            "CONVERT(`price` USING utf8mb4) AS `price`"
        );
    }

    // ── QueryDialect — is_native_type ──────────────────────────────────────

    #[test]
    fn test_postgres_native_types() {
        let d = PostgresDialect;
        assert!(d.is_native_type("boolean"));
        assert!(d.is_native_type("integer"));
        assert!(d.is_native_type("bigint"));
        assert!(!d.is_native_type("numeric"));
        assert!(!d.is_native_type("varchar"));
        assert!(!d.is_native_type("json"));
    }

    #[test]
    fn test_mysql_native_types() {
        let d = MysqlDialect;
        assert!(d.is_native_type("int"));
        assert!(!d.is_native_type("tinyint"));
        assert!(d.is_native_type("double"));
        assert!(!d.is_native_type("decimal"));
        assert!(!d.is_native_type("varchar"));
        assert!(!d.is_native_type("json"));
        assert!(!d.is_native_type("date"));
    }

    // ── QueryDialect — sql_literal ─────────────────────────────────────────

    #[test]
    fn test_sql_literal_null() {
        assert_eq!(PostgresDialect.sql_literal(&Value::Null), "NULL");
        assert_eq!(MysqlDialect.sql_literal(&Value::Null), "NULL");
    }

    #[test]
    fn test_sql_literal_bool() {
        assert_eq!(PostgresDialect.sql_literal(&Value::Bool(true)), "TRUE");
        assert_eq!(MysqlDialect.sql_literal(&Value::Bool(false)), "FALSE");
    }

    #[test]
    fn test_sql_literal_string_escapes() {
        let v = Value::String("it's fine".to_string());
        assert_eq!(PostgresDialect.sql_literal(&v), "'it''s fine'");
        assert_eq!(MysqlDialect.sql_literal(&v), "'it''s fine'");
    }

    #[test]
    fn test_sql_literal_json_postgres_has_cast() {
        let v = serde_json::json!({"k": "v"});
        let lit = PostgresDialect.sql_literal(&v);
        assert!(lit.ends_with("::jsonb"), "Expected ::jsonb, got: {}", lit);
    }

    #[test]
    fn test_sql_literal_json_mysql_no_cast() {
        let v = serde_json::json!({"k": "v"});
        let lit = MysqlDialect.sql_literal(&v);
        assert!(
            !lit.contains("::"),
            "MySQL must not have any cast, got: {}",
            lit
        );
        assert!(lit.starts_with('\''));
    }

    #[test]
    fn test_sql_literal_json_sqlite_no_cast() {
        let v = serde_json::json!([1, 2, 3]);
        let lit = SqliteDialect.sql_literal(&v);
        assert!(
            !lit.contains("::"),
            "SQLite must not have any cast, got: {}",
            lit
        );
    }

    // ── QueryDialect — needs_introspection ─────────────────────────────────

    #[test]
    fn test_needs_introspection() {
        assert!(PostgresDialect.needs_introspection());
        assert!(MysqlDialect.needs_introspection());
        assert!(MariadbDialect.needs_introspection());
        assert!(!SqliteDialect.needs_introspection());
    }

    // ── Factory ────────────────────────────────────────────────────────────

    #[test]
    fn test_from_driver_names() {
        assert_eq!(from_driver("postgres").name(), "postgres");
        assert_eq!(from_driver("mysql").name(), "mysql");
        assert_eq!(from_driver("mariadb").name(), "mariadb");
        assert_eq!(from_driver("sqlite").name(), "sqlite");
        assert_eq!(from_driver("unknown").name(), "postgres"); // default
    }
}
