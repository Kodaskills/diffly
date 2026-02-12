use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::any::AnyPoolOptions;
use sqlx::AnyPool;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::debug;

use crate::domain::ports::RowRepository;
use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::{ColumnName, ExcludedColumns, Schema, TableName};
use crate::infrastructure::config::DbConfig;
use crate::infrastructure::db::dialect::{from_driver, Dialect};
use crate::infrastructure::db::row_mapper::row_to_map;
use crate::infrastructure::db::sql_utils::{build_select_query, build_typed_select_query};

pub struct SqlxRowRepository {
    pool: AnyPool,
    dialect: Arc<dyn Dialect>,
}

/// Connect to the database described in `cfg` and return a `SqlxRowRepository`.
pub async fn connect(cfg: &DbConfig) -> Result<SqlxRowRepository> {
    sqlx::any::install_default_drivers();

    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.url())
        .await
        .with_context(|| {
            format!(
                "Failed to connect to {} (driver: {})",
                cfg.dbname, cfg.driver
            )
        })?;

    debug!(
        "Connected to {}/{} via {} driver",
        cfg.host, cfg.dbname, cfg.driver
    );

    Ok(SqlxRowRepository {
        pool,
        dialect: Arc::from(from_driver(&cfg.driver)),
    })
}

/// Read a column from an AnyRow as String, handling MySQL's habit of returning
/// information_schema string columns as BLOB to sqlx AnyRow.
fn blob_or_string(row: &sqlx::any::AnyRow, idx: usize) -> Result<String> {
    use sqlx::{Column, Row, TypeInfo};
    let type_name = row.column(idx).type_info().name();
    if type_name == "BLOB" {
        let bytes: Vec<u8> = row.try_get(idx)?;
        Ok(String::from_utf8(bytes).unwrap_or_default())
    } else {
        Ok(row.try_get(idx)?)
    }
}

/// Query `information_schema.columns` for `(column_name, data_type)` pairs.
/// The SQL and placeholders are provided by the dialect.
async fn fetch_column_types(
    pool: &AnyPool,
    schema: &Schema,
    table: &TableName,
    dialect: &dyn Dialect,
) -> Result<Vec<(String, String)>> {
    let sql = dialect.introspect_sql();

    let rows = sqlx::query(sql)
        .bind(&schema.0)
        .bind(&table.0)
        .fetch_all(pool)
        .await
        .with_context(|| format!("Failed to fetch column types for {}.{}", schema.0, table.0))?;

    let mut cols = Vec::with_capacity(rows.len());
    for row in &rows {
        // MySQL/MariaDB returns information_schema strings as BLOB — handle both.
        let col_name = blob_or_string(row, 0)?;
        let data_type = blob_or_string(row, 1)?;
        cols.push((col_name, data_type));
    }
    Ok(cols)
}

#[async_trait]
impl RowRepository for SqlxRowRepository {
    async fn fetch_rows(
        &self,
        schema: &Schema,
        table: &TableName,
        pk_cols: &[ColumnName],
        excluded: &ExcludedColumns,
    ) -> Result<Vec<RowMap>> {
        // Dialects that support information_schema introspection (Postgres, MySQL,
        // MariaDB) use a typed SELECT where unsupported column types are cast to
        // text, and the mapper reconstructs the correct Value variant from the
        // type hint. Dialects without introspection (SQLite) use SELECT * —
        // SQLite's loose affinity means AnyRow decodes all storage classes natively.
        let (query, col_types_map) = if self.dialect.needs_introspection() {
            let col_types =
                fetch_column_types(&self.pool, schema, table, self.dialect.as_ref()).await?;
            let q =
                build_typed_select_query(schema, table, pk_cols, &col_types, self.dialect.as_ref());
            let type_map: BTreeMap<String, String> = col_types.into_iter().collect();
            (q, type_map)
        } else {
            (
                build_select_query(schema, table, pk_cols, self.dialect.as_ref()),
                BTreeMap::new(),
            )
        };

        debug!("Executing: {}", query);

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Failed to query {}.{}", schema.0, table.0))?;

        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut map = row_to_map(row, &col_types_map, self.dialect.as_ref())?;
            for col in &excluded.0 {
                map.remove(col);
            }
            result.push(map);
        }
        Ok(result)
    }
}
