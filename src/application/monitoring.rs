use crate::domain::ports::{Differ, RowRepository};
use crate::domain::{
    table_diff::{RowMap, TableDiff},
    value_objects::{ColumnName, ExcludedColumns, Schema, TableName},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{info, instrument};

// ─── PerfReport ──────────────────────────────────────────────────────────────

/// A single timed operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpTiming {
    /// Operation name: "fetch_rows" or "diff_table".
    pub operation: &'static str,
    /// Table this operation was performed on.
    pub table: String,
    /// Elapsed wall time in milliseconds.
    pub duration_ms: u128,
    /// Number of rows involved (fetched or diffed).
    pub rows: usize,
}

/// Accumulated performance timings for a single diffly run.
///
/// Shared across all decorator instances for one run via `Arc<Mutex<_>>`.
/// After the run, pass to [`crate::presentation::cli_summary::print_perf_summary`]
/// to render a human-readable table.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PerfReport {
    pub timings: Vec<OpTiming>,
    pub total_rows_fetched: usize,
    pub total_ms: u128,
}

impl PerfReport {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    fn record(report: &Arc<Mutex<Self>>, timing: OpTiming) {
        if let Ok(mut r) = report.lock() {
            r.total_ms += timing.duration_ms;
            if timing.operation == "fetch_rows" {
                r.total_rows_fetched += timing.rows;
            }
            r.timings.push(timing);
        }
    }
}

// ─── MonitoringRowRepository ─────────────────────────────────────────────────

/// Decorator: wraps any `RowRepository`, measures wall time per `fetch_rows`
/// call, and appends the result to the shared `PerfReport`.
pub struct MonitoringRowRepository {
    inner: Arc<dyn RowRepository>,
    report: Arc<Mutex<PerfReport>>,
}

impl MonitoringRowRepository {
    pub fn new(inner: Arc<dyn RowRepository>, report: Arc<Mutex<PerfReport>>) -> Self {
        Self { inner, report }
    }
}

#[async_trait]
impl RowRepository for MonitoringRowRepository {
    #[instrument(
        name = "fetch_rows",
        skip(self, schema, table, pk_cols, excluded),
        fields(db.schema = %schema.0, db.table = %table.0),
        level = "info"
    )]
    async fn fetch_rows(
        &self,
        schema: &Schema,
        table: &TableName,
        pk_cols: &[ColumnName],
        excluded: &ExcludedColumns,
    ) -> Result<Vec<RowMap>> {
        let start = Instant::now();
        let rows = self
            .inner
            .fetch_rows(schema, table, pk_cols, excluded)
            .await?;
        let duration_ms = start.elapsed().as_millis();

        info!(table = %table.0, rows = rows.len(), duration_ms, "fetch_rows completed");

        PerfReport::record(
            &self.report,
            OpTiming {
                operation: "fetch_rows",
                table: table.0.clone(),
                duration_ms,
                rows: rows.len(),
            },
        );

        Ok(rows)
    }
}

// ─── MonitoringDiffer ────────────────────────────────────────────────────────

/// Decorator: wraps any `Differ`, measures wall time per `diff_table` call,
/// and appends the result to the shared `PerfReport`.
pub struct MonitoringDiffer {
    inner: Arc<dyn Differ>,
    report: Arc<Mutex<PerfReport>>,
}

impl MonitoringDiffer {
    pub fn new(inner: Arc<dyn Differ>, report: Arc<Mutex<PerfReport>>) -> Self {
        Self { inner, report }
    }
}

impl Differ for MonitoringDiffer {
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
        let start = Instant::now();
        let result = self.inner.diff_table(source, target, pk_cols, table_name);
        let duration_ms = start.elapsed().as_millis();

        let changes = result.inserts.len() + result.updates.len() + result.deletes.len();
        info!(table = %table_name.0, source_rows = source.len(), target_rows = target.len(), changes, duration_ms, "diff_table completed");

        PerfReport::record(
            &self.report,
            OpTiming {
                operation: "diff_table",
                table: table_name.0.clone(),
                duration_ms,
                rows: source.len() + target.len(),
            },
        );

        result
    }
}
