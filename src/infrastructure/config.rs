use anyhow::{Context, Result};
use serde::Deserialize;
use toml;

use crate::domain::value_objects::ExcludedColumns;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub source: DbConfig,
    pub target: DbConfig,
    pub diff: DiffConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbConfig {
    /// Database driver: "postgres" (default), "mysql", "mariadb", or "sqlite".
    #[serde(default = "default_driver")]
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub user: String,
    pub password: String,
    pub schema: String,
}

fn default_driver() -> String {
    "postgres".to_string()
}

#[derive(Debug, Deserialize)]
pub struct DiffConfig {
    pub tables: Vec<TableConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TableConfig {
    pub name: String,
    pub primary_key: Vec<String>,
    #[serde(default)]
    pub excluded_columns: ExcludedColumns,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub dir: String,
}

impl DbConfig {
    /// Build a sqlx-compatible connection URL from this config.
    pub fn url(&self) -> String {
        match self.driver.as_str() {
            "mysql" | "mariadb" => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.dbname
            ),
            "sqlite" => format!("sqlite://{}", self.dbname),
            _ => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.dbname
            ),
        }
    }
}

impl AppConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        let cfg: AppConfig =
            toml::from_str(&content).with_context(|| "Failed to parse config TOML")?;
        Ok(cfg)
    }
}
