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
    /// Percent-encode a string for safe use in a connection URL.
    fn encode(s: &str) -> String {
        let mut encoded = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                // Unreserved characters — safe as-is
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
                // Everything else gets percent-encoded
                c => {
                    let mut buf = [0u8; 4];
                    let bytes = c.encode_utf8(&mut buf);
                    for byte in bytes.bytes() {
                        encoded.push('%');
                        encoded.push_str(&format!("{:02X}", byte));
                    }
                }
            }
        }
        encoded
    }

    /// Build a sqlx-compatible connection URL from this config.
    pub fn url(&self) -> String {
        let user = Self::encode(&self.user);
        let password = Self::encode(&self.password);
        match self.driver.as_str() {
            "mysql" | "mariadb" => format!(
                "mysql://{}:{}@{}:{}/{}",
                user, password, self.host, self.port, self.dbname
            ),
            "sqlite" => format!("sqlite://{}", self.dbname),
            _ => format!(
                "postgres://{}:{}@{}:{}/{}",
                user, password, self.host, self.port, self.dbname
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
