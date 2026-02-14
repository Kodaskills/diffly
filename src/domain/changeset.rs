use crate::domain::table_diff::TableDiff;
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
pub struct Changeset {
    pub changeset_id: String,
    pub source_schema: String,
    pub target_schema: String,
    /// Database driver used to produce this changeset: "postgres", "mysql", "mariadb", "sqlite".
    /// Used by SqlWriter to generate the correct SQL dialect.
    #[serde(default = "default_driver")]
    pub driver: String,
    pub created_at: String,
    pub source_fingerprint: String,
    pub target_fingerprint: String,
    pub tables: Vec<TableDiff>,
    pub summary: Summary,
}

#[allow(dead_code)] // invoked by serde(default), not called directly
fn default_driver() -> String {
    "postgres".to_string()
}

#[derive(Debug, Serialize, Clone)]
pub struct Summary {
    pub total_inserts: usize,
    pub total_updates: usize,
    pub total_deletes: usize,
    pub total_changes: usize,
    pub tables_affected: usize,
}

impl Changeset {
    pub fn new(
        source_schema: &str,
        target_schema: &str,
        driver: &str,
        tables: Vec<TableDiff>,
    ) -> Self {
        let total_inserts: usize = tables.iter().map(|t| t.inserts.len()).sum();
        let total_updates: usize = tables.iter().map(|t| t.updates.len()).sum();
        let total_deletes: usize = tables.iter().map(|t| t.deletes.len()).sum();
        let tables_affected = tables.iter().filter(|t| !t.is_empty()).count();

        Changeset {
            changeset_id: format!(
                "cs_{}_{}",
                Utc::now().format("%Y%m%d_%H%M%S"),
                Uuid::new_v4().simple()
            ),
            source_schema: source_schema.to_string(),
            target_schema: target_schema.to_string(),
            driver: driver.to_string(),
            created_at: Utc::now().to_rfc3339(),
            source_fingerprint: String::new(), // Computed during diff if needed
            target_fingerprint: String::new(),
            tables,
            summary: Summary {
                total_inserts,
                total_updates,
                total_deletes,
                total_changes: total_inserts + total_updates + total_deletes,
                tables_affected,
            },
        }
    }
}
