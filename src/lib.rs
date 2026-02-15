use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

// ─── Log level ────────────────────────────────────────────────────────────────

/// Controls the verbosity of diffly's internal tracing output.
///
/// Pass to [`init_tracing`] before calling any async entry point.
///
/// | Variant | `tracing` level | When to use                         |
/// |---------|-----------------|-------------------------------------|
/// | `Error` | `error`         | `--quiet` / CI scripting            |
/// | `Info`  | `info`          | Default — shows per-table timings   |
/// | `Debug` | `debug`         | `--verbose` — shows SQL queries too |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Error,
    #[default]
    Info,
    Debug,
}

/// Initialise the global `tracing` subscriber for diffly.
///
/// This is a convenience wrapper around `tracing_subscriber`. It respects
/// `RUST_LOG` when set, falling back to `level` otherwise.
///
/// Call this **once** at application startup, before any diffly async function.
/// Library consumers who manage their own subscriber should skip this and
/// configure tracing themselves.
///
/// Only available when the `cli` feature is enabled (pulls in
/// `tracing-subscriber`).
#[cfg(feature = "cli")]
pub fn init_tracing(level: LogLevel) {
    use tracing_subscriber::fmt::format::FmtSpan;

    let default_filter = match level {
        LogLevel::Error => "diffly=error",
        LogLevel::Info  => "diffly=info",
        LogLevel::Debug => "diffly=debug",
    };

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();
}

// ─── Public API Facade ───

pub use application::monitoring::PerfReport;
pub use domain::changeset::{Changeset, Summary};
pub use domain::conflict::ConflictReport;
pub use domain::diff_result::DiffResult;
pub use domain::fingerprint::fingerprint;
pub use domain::ports::SnapshotProvider;
pub use domain::snapshot::MapSnapshotProvider;
pub use domain::table_diff::{ColumnDiff, RowChange, RowMap, RowUpdate, TableDiff};
pub use domain::value_objects::{ColumnName, ExcludedColumns, Fingerprint, Schema, TableName};
pub use infrastructure::config::{AppConfig, DbConfig, DiffConfig, OutputConfig, TableConfig};

use crate::application::conflict::ConflictService;
use crate::application::diff::{DiffService, TableDiffer};
use crate::application::monitoring::{MonitoringDiffer, MonitoringRowRepository};
use crate::application::snapshot::SnapshotService;
use crate::domain::ports::RowRepository;
use crate::infrastructure::db::client::connect;

// ─── Public entry points ───

/// 2-way diff only (no conflict detection).
///
/// Returns the raw `Changeset` (source vs. current target).
/// Use [`run_with_conflicts`] if you need the 3-way merge.
/// Use [`run_with_timing`] if you also want a performance report.
pub async fn run(cfg: &AppConfig) -> Result<Changeset> {
    let (changeset, _) = run_with_timing(cfg).await?;
    Ok(changeset)
}

/// 2-way diff with performance timing.
///
/// Returns the `Changeset` and a [`PerfReport`] containing per-table
/// fetch and diff timings.
pub async fn run_with_timing(cfg: &AppConfig) -> Result<(Changeset, PerfReport)> {
    let report = PerfReport::new();

    let source_repo = build_repo(&cfg.source, Arc::clone(&report)).await?;
    let target_repo = build_repo(&cfg.target, Arc::clone(&report)).await?;
    let differ = Arc::new(MonitoringDiffer::new(
        Arc::new(TableDiffer::new()),
        Arc::clone(&report),
    ));

    let service = DiffService::new(source_repo, target_repo, differ);

    let source_schema = Schema(cfg.source.schema.clone());
    let target_schema = Schema(cfg.target.schema.clone());

    let changeset = service
        .run_diff(
            &source_schema,
            &target_schema,
            &cfg.source.driver,
            &cfg.diff.tables,
        )
        .await?;

    let perf = report.lock().unwrap().clone();
    Ok((changeset, perf))
}

/// Capture a point-in-time snapshot of the **target** DB for all configured tables.
///
/// Call this at **source-clone time**. The returned map is the data you
/// should serialise (JSON, DynamoDB, S3…) and restore via [`snapshot_provider`].
/// ```
pub async fn snapshot(cfg: &AppConfig) -> Result<BTreeMap<String, Vec<RowMap>>> {
    let (raw, _) = snapshot_with_timing(cfg).await?;
    Ok(raw)
}

/// Capture a snapshot and return a [`PerfReport`] alongside the rows.
pub async fn snapshot_with_timing(
    cfg: &AppConfig,
) -> Result<(BTreeMap<String, Vec<RowMap>>, PerfReport)> {
    let report = PerfReport::new();
    let target_repo = build_repo(&cfg.target, Arc::clone(&report)).await?;
    let svc = SnapshotService::new(target_repo);
    let target_schema = Schema(cfg.target.schema.clone());
    let raw = svc.capture(&target_schema, &cfg.diff.tables).await?;
    let perf = report.lock().unwrap().clone();
    Ok((raw, perf))
}

/// Wrap a previously-captured snapshot map as a [`SnapshotProvider`].
///
/// Counterpart of [`snapshot`]: deserialise the stored JSON back into
/// `BTreeMap<String, Vec<RowMap>>` and pass it here. The returned value is
/// ready to use as the `base` argument to [`run_with_conflicts`].
pub fn snapshot_provider(data: BTreeMap<String, Vec<RowMap>>) -> MapSnapshotProvider {
    MapSnapshotProvider::new(data)
}

/// 2-way diff + 3-way conflict detection.
///
/// # Arguments
/// * `cfg`                 — application configuration (same as [`run`])
/// * `base`                — snapshot of target at **source-clone time**,
///                           obtained via [`snapshot_provider`]
/// * `stored_fps`          — per-table SHA-256 fingerprints stored at clone time
/// * `current_target_rows` — current target rows per table
///
/// The caller is responsible for persisting and restoring `base` and
/// `stored_fps`. Diffly has no opinion on storage.
pub async fn run_with_conflicts(
    cfg: &AppConfig,
    base: &dyn SnapshotProvider,
    stored_fps: &BTreeMap<String, Fingerprint>,
    current_target_rows: &BTreeMap<String, Vec<RowMap>>,
) -> Result<DiffResult> {
    let changeset = run(cfg).await?;

    let pk_cols_by_table: BTreeMap<String, Vec<ColumnName>> = cfg
        .diff
        .tables
        .iter()
        .map(|t| {
            let cols = t
                .primary_key
                .iter()
                .map(|pk| ColumnName(pk.clone()))
                .collect();
            (t.name.clone(), cols)
        })
        .collect();

    let conflict_svc = ConflictService::new();
    Ok(conflict_svc.check(
        changeset,
        base,
        stored_fps,
        current_target_rows,
        &pk_cols_by_table,
    ))
}

// ─── Private helpers ───────────────────────────────────────────────────────────

/// Connect to a DB and wrap the repository in the monitoring decorator.
///
/// The shared `report` accumulates timings from all repos created for the
/// same run, giving a unified view across source and target.
async fn build_repo(
    cfg: &DbConfig,
    report: Arc<std::sync::Mutex<PerfReport>>,
) -> Result<Arc<dyn RowRepository>> {
    let repo = Arc::new(connect(cfg).await?);
    Ok(Arc::new(MonitoringRowRepository::new(repo, report)))
}
