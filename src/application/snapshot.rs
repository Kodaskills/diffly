use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::domain::ports::RowRepository;
use crate::domain::table_diff::RowMap;
use crate::domain::value_objects::{ColumnName, Schema, TableName};
use crate::infrastructure::config::TableConfig;

// ─────────────────────────────────────────────────────────────────────────────
// SnapshotService
// ─────────────────────────────────────────────────────────────────────────────

/// Fetches the current state of the target database for all configured tables.
///
/// # Responsibility (SRP)
/// `DiffService` computes a diff between source and target.
/// `SnapshotService` captures a point-in-time snapshot of the **target** DB
/// only. It has one reason to change: the snapshot-capture logic.
///
/// # Usage
/// Call this at **source-clone time** to record target's current state.
/// The returned `BTreeMap<table_name, Vec<RowMap>>` is the raw data the
/// orchestrator should serialise (JSON/DynamoDB/S3) and pass back to
/// `run_with_conflicts` at deploy time via `snapshot_provider()`.
pub struct SnapshotService {
    target_repo: Arc<dyn RowRepository>,
}

impl SnapshotService {
    pub fn new(target_repo: Arc<dyn RowRepository>) -> Self {
        Self { target_repo }
    }

    /// Fetch all rows from the target DB for every configured table, in parallel.
    ///
    /// Returns a map of `table_name → Vec<RowMap>` ready to be serialised by
    /// the orchestrator and later restored via `diffly::snapshot_provider()`.
    pub async fn capture(
        &self,
        target_schema: &Schema,
        tables: &[TableConfig],
    ) -> Result<BTreeMap<String, Vec<RowMap>>> {
        let mut handles = Vec::with_capacity(tables.len());

        for table_cfg in tables {
            let repo = Arc::clone(&self.target_repo);
            let schema = target_schema.clone();
            let table_cfg = table_cfg.clone();

            let handle = tokio::spawn(async move {
                let table_name = TableName(table_cfg.name.clone());
                let pk_cols: Vec<ColumnName> = table_cfg
                    .primary_key
                    .iter()
                    .map(|pk| ColumnName(pk.clone()))
                    .collect();

                let rows = repo
                    .fetch_rows(&schema, &table_name, &pk_cols, &table_cfg.excluded_columns)
                    .await?;

                Ok::<_, anyhow::Error>((table_cfg.name.clone(), rows))
            });

            handles.push(handle);
        }

        let mut snapshot = BTreeMap::new();
        for handle in handles {
            let (table_name, rows) = handle.await??;
            snapshot.insert(table_name, rows);
        }

        Ok(snapshot)
    }
}
