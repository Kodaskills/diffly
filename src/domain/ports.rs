use crate::domain::{
    changeset::Changeset,
    table_diff::{RowMap, TableDiff},
    value_objects::{ColumnName, ExcludedColumns, Schema, TableName},
};
use anyhow::Result;
use async_trait::async_trait;

/// Port: access to data in a table (implemented by SqlxRowRepository)
#[async_trait]
pub trait RowRepository: Send + Sync {
    async fn fetch_rows(
        &self,
        schema: &Schema,
        table: &TableName,
        pk_cols: &[ColumnName],
        excluded: &ExcludedColumns,
    ) -> Result<Vec<RowMap>>;
}

/// Port: table diff algorithm (implemented by TableDiffer)
pub trait Differ: Send + Sync {
    fn diff_table(
        &self,
        source: &[RowMap],
        target: &[RowMap],
        pk_cols: &[ColumnName],
        table_name: &TableName,
    ) -> TableDiff;
}

/// Port: output formatting (implemented by JsonWriter, SqlWriter, HtmlWriter)
pub trait OutputWriter: Send + Sync {
    /// Serializes the changeset to a string (JSON, SQL, HTML, etc.)
    fn format(&self, changeset: &Changeset) -> Result<String>;
    /// Extension of the produced file (e.g. "json", "sql", "html")
    fn extension(&self) -> &'static str;
}
