use anyhow::Result;
use std::sync::Arc;

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

// ─── Public API Facade ───

pub use domain::changeset::{Changeset, Summary};
pub use domain::table_diff::{ColumnDiff, RowChange, RowMap, RowUpdate, TableDiff};
pub use domain::value_objects::{ColumnName, ExcludedColumns, Schema, TableName};
pub use infrastructure::config::{AppConfig, DbConfig, DiffConfig, OutputConfig, TableConfig};

use crate::application::diff::{DiffService, TableDiffer};
use crate::application::monitoring::{MonitoringDiffer, MonitoringRowRepository};
use crate::infrastructure::db::client::connect;

/// High-level entry point for workspace consumers.
pub async fn run(cfg: &AppConfig) -> Result<Changeset> {
    // 1. Assemble dependencies from the infrastructure layer
    let source_repo = Arc::new(connect(&cfg.source).await?);
    let target_repo = Arc::new(connect(&cfg.target).await?);
    let differ = Arc::new(TableDiffer::new());

    // --- Decorate with monitoring ---
    let monitored_source_repo = Arc::new(MonitoringRowRepository::new(source_repo));
    let monitored_target_repo = Arc::new(MonitoringRowRepository::new(target_repo));
    let monitored_differ = Arc::new(MonitoringDiffer::new(differ));

    // 2. Create the application service with monitored components
    let service = DiffService::new(
        monitored_source_repo,
        monitored_target_repo,
        monitored_differ,
    );

    // 3. Run the diff by delegating to the service
    let source_schema = Schema(cfg.source.schema.clone());
    let target_schema = Schema(cfg.target.schema.clone());

    // Source and target should use the same driver; use source driver for SQL output dialect.
    service
        .run_diff(
            &source_schema,
            &target_schema,
            &cfg.source.driver,
            &cfg.diff.tables,
        )
        .await
}
