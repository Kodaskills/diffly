use anyhow::Result;
use sqlx::any::AnyRow;
use sqlx::{Column, Row, TypeInfo};
use std::collections::BTreeMap;

use crate::domain::table_diff::RowMap;
use crate::infrastructure::db::dialect::RowDecoder;

/// Convert a sqlx `AnyRow` into a `RowMap`.
///
/// `col_types` maps column names to their `information_schema.data_type` values.
/// `decoder` is the dialect-specific `RowDecoder` that knows how to turn an
/// AnyRow column index + type hint into the correct `serde_json::Value`.
pub fn row_to_map(
    row: &AnyRow,
    col_types: &BTreeMap<String, String>,
    decoder: &dyn RowDecoder,
) -> Result<RowMap> {
    let mut map = BTreeMap::new();
    for col in row.columns() {
        let name = col.name().to_string();
        // Prefer the information_schema type hint (more precise than AnyRow's
        // runtime type name). Fall back to the AnyRow type name for SQLite where
        // col_types is empty.
        let anyrow_type = col.type_info().name();
        let type_hint = col_types
            .get(&name)
            .map(|s| s.as_str())
            .unwrap_or(anyrow_type);

        let value = decoder.decode_column(row, col.ordinal(), type_hint)?;
        map.insert(name, value);
    }
    Ok(map)
}
