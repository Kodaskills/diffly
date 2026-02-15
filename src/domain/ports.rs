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

/// Port: provides the base snapshot of a table taken at source-clone time.
///
/// The base snapshot is the state of target **at the moment the source was
/// cloned**. It is stored externally (S3, DynamoDB, local file) by the
/// orchestrator and injected into `ConflictService` at deploy time.
///
/// Implementations are provided by the caller — diffly has no knowledge of
/// where or how the snapshot is persisted.
///
/// Returns `None` for a given table when no snapshot exists, in which case
/// `ConflictService` skips the 3-way merge for that table (treats it as clean).
pub trait SnapshotProvider: Send + Sync {
    fn get(&self, table: &TableName) -> Option<&[RowMap]>;
}
