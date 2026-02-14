use crate::domain::ports::{Differ, RowRepository};
use crate::domain::{
    table_diff::{RowMap, TableDiff},
    value_objects::{ColumnName, ExcludedColumns, Schema, TableName},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::instrument;

// --- Monitoring Decorator for RowRepository ---

pub struct MonitoringRowRepository<R: RowRepository> {
    inner: Arc<R>,
}

impl<R: RowRepository> MonitoringRowRepository<R> {
    pub fn new(inner: Arc<R>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R: RowRepository> RowRepository for MonitoringRowRepository<R> {
    #[instrument(
        name = "fetch_rows",
        skip(self, schema, table, pk_cols, excluded),
        fields(
            db.schema = %schema.0,
            db.table = %table.0
        ),
        level = "info"
    )]
    async fn fetch_rows(
        &self,
        schema: &Schema,
        table: &TableName,
        pk_cols: &[ColumnName],
        excluded: &ExcludedColumns,
    ) -> Result<Vec<RowMap>> {
        self.inner
            .fetch_rows(schema, table, pk_cols, excluded)
            .await
    }
}

// --- Monitoring Decorator for Differ ---

pub struct MonitoringDiffer<D: Differ> {
    inner: Arc<D>,
}

impl<D: Differ> MonitoringDiffer<D> {
    pub fn new(inner: Arc<D>) -> Self {
        Self { inner }
    }
}

impl<D: Differ> Differ for MonitoringDiffer<D> {
    #[instrument(
        name = "diff_table",
        skip(self, source, target, pk_cols, table_name),
        fields(
            db.table = %table_name.0,
            source.rows = source.len(),
            target.rows = target.len(),
        ),
        level = "info"
    )]
    fn diff_table(
        &self,
        source: &[RowMap],
        target: &[RowMap],
        pk_cols: &[ColumnName],
        table_name: &TableName,
    ) -> TableDiff {
        self.inner.diff_table(source, target, pk_cols, table_name)
    }
}
