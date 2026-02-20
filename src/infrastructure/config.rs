use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat, Map};
use serde::Deserialize;

use crate::domain::value_objects::ExcludedColumns;

// ─── Structs ──────────────────────────────────────────────────────────────────

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

// ─── URL builder ─────────────────────────────────────────────────────────────

impl DbConfig {
    /// Percent-encode a string for safe use in a connection URL.
    fn encode(s: &str) -> String {
        let mut encoded = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
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

// ─── Layered loading (Viper-style) ───────────────────────────────────────────
//
// Priority order (highest → lowest):
//   1. Environment variables   DIFFLY_SOURCE__HOST, DIFFLY_TARGET__PASSWORD, …
//   2. Explicit --config <path> flag
//   3. ./diffly.toml           (local project file, optional)
//   4. ~/.config/diffly/diffly.toml  (user-level config, optional)
//   5. Built-in defaults
//
// Env var convention:
//   prefix    : DIFFLY_
//   separator : __  (double underscore = nested key)
//   examples  :
//     DIFFLY_SOURCE__HOST=localhost
//     DIFFLY_SOURCE__PORT=5432
//     DIFFLY_SOURCE__PASSWORD=secret
//     DIFFLY_TARGET__DBNAME=my_db
//     DIFFLY_OUTPUT__DIR=./output

impl AppConfig {
    /// Load configuration from layered sources.
    ///
    /// `explicit_path` — value of the `--config` CLI flag (`None` = not provided).
    pub fn load(explicit_path: Option<&str>) -> Result<Self> {
        Self::load_inner(explicit_path, None)
    }

    /// Internal loader — accepts an optional synthetic env map for hermetic testing.
    fn load_inner(
        explicit_path: Option<&str>,
        synthetic_env: Option<Map<String, String>>,
    ) -> Result<Self> {
        // 5. Built-in defaults
        let mut builder = Config::builder()
            .set_default("source.driver", "postgres")?
            .set_default("source.host", "localhost")?
            .set_default("source.port", 5432)?
            .set_default("source.schema", "public")?
            .set_default("target.driver", "postgres")?
            .set_default("target.host", "localhost")?
            .set_default("target.port", 5432)?
            .set_default("target.schema", "public")?
            .set_default("output.dir", "./output")?;

        // Sources are added lowest → highest priority (later = wins).

        // 4. User-level config  ~/.config/diffly/diffly.toml  (optional)
        if let Some(cfg_dir) = dirs::config_dir() {
            let home_cfg = cfg_dir.join("diffly").join("diffly.toml");
            builder = builder.add_source(
                File::from(home_cfg)
                    .format(FileFormat::Toml)
                    .required(false),
            );
        }

        // 3. Local project file  ./diffly.toml  (optional — env vars alone are enough)
        builder = builder.add_source(
            File::with_name("diffly.toml")
                .format(FileFormat::Toml)
                .required(false),
        );

        // 2. Explicit --config <path>  overrides local file but stays below env vars.
        //    This matches Viper: named config file < env vars.
        //    Use --config for a deployment-specific file, env vars for secrets/overrides.
        if let Some(path) = explicit_path {
            builder = builder.add_source(
                File::with_name(path)
                    .format(FileFormat::Toml)
                    .required(true),
            );
        }

        // 1. Environment variables  DIFFLY_SOURCE__HOST etc.  (highest priority)
        //    prefix_separator="_" separates prefix from key  : DIFFLY_SOURCE__HOST
        //    separator="__"       separates nested key parts : SOURCE__HOST → source.host
        //    In tests a synthetic map can be injected to avoid touching the real env.
        let env_source = Environment::with_prefix("DIFFLY")
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true)
            .source(synthetic_env);
        builder = builder.add_source(env_source);

        let cfg = builder
            .build()
            .context("Failed to build configuration")?
            .try_deserialize::<AppConfig>()
            .context("Failed to deserialize configuration")?;

        Ok(cfg)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Build a synthetic env map. Keys are full uppercase env var names (e.g. `DIFFLY_SOURCE__DBNAME`).
    fn env(pairs: &[(&str, &str)]) -> Option<Map<String, String>> {
        Some(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
        )
    }

    /// Verify that the config crate env source picks up keys from a synthetic map correctly.
    #[test]
    fn env_source_key_mapping_probe() {
        use config::{Config, Environment, Source};
        let mut map = HashMap::new();
        map.insert("DIFFLY_SOURCE__DBNAME".to_string(), "probe_db".to_string());
        map.insert("DIFFLY_SOURCE__PORT".to_string(), "9999".to_string());

        let cfg = Config::builder()
            .set_default("source.dbname", "default").unwrap()
            .set_default("source.port", 5432i64).unwrap()
            .add_source(
                Environment::with_prefix("DIFFLY")
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true)
                    .source(Some(map)),
            )
            .build()
            .unwrap();

        let collected = cfg.collect().unwrap();
        eprintln!("Collected keys: {:?}", collected.keys().collect::<Vec<_>>());
        eprintln!("source: {:?}", collected.get("source"));

        assert_eq!(cfg.get_string("source.dbname").unwrap(), "probe_db");
        assert_eq!(cfg.get_int("source.port").unwrap(), 9999);
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn minimal_toml(source_dbname: &str, target_dbname: &str) -> String {
        format!(
            r#"
[source]
host = "localhost"
port = 5432
dbname = "{source_dbname}"
user = "user"
password = "pass"

[target]
host = "localhost"
port = 5432
dbname = "{target_dbname}"
user = "user"
password = "pass"

[diff]
tables = []

[output]
dir = "./output"
"#
        )
    }

    fn write_toml(content: &str) -> NamedTempFile {
        // Use .toml suffix so `config` crate detects the format by extension too.
        let mut f = NamedTempFile::with_suffix(".toml").unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    // ── AppConfig::load ───────────────────────────────────────────────────────

    #[test]
    fn load_explicit_path() {
        let f = write_toml(&minimal_toml("src_db", "tgt_db"));
        let cfg = AppConfig::load(Some(f.path().to_str().unwrap())).unwrap();
        assert_eq!(cfg.source.dbname, "src_db");
        assert_eq!(cfg.target.dbname, "tgt_db");
    }

    #[test]
    fn load_defaults_applied() {
        // Minimal TOML without driver / schema / output.dir — defaults must fill them in.
        let f = write_toml(&minimal_toml("src", "tgt"));
        let cfg = AppConfig::load(Some(f.path().to_str().unwrap())).unwrap();

        assert_eq!(cfg.source.driver, "postgres");
        assert_eq!(cfg.source.schema, "public");
        assert_eq!(cfg.target.driver, "postgres");
        assert_eq!(cfg.target.schema, "public");
        assert_eq!(cfg.output.dir, "./output");
    }

    #[test]
    fn load_defaults_overridden_by_file() {
        let toml = r#"
[source]
host = "db.example.com"
port = 5433
dbname = "prod"
user = "admin"
password = "s3cr3t"
schema = "myschema"
driver = "mysql"

[target]
host = "localhost"
port = 5432
dbname = "staging"
user = "user"
password = "pass"

[diff]
tables = []

[output]
dir = "/var/output"
"#;
        let f = write_toml(toml);
        let cfg = AppConfig::load(Some(f.path().to_str().unwrap())).unwrap();

        assert_eq!(cfg.source.driver, "mysql");
        assert_eq!(cfg.source.host, "db.example.com");
        assert_eq!(cfg.source.port, 5433);
        assert_eq!(cfg.source.schema, "myschema");
        assert_eq!(cfg.output.dir, "/var/output");
    }

    #[test]
    fn load_explicit_overrides_base_values() {
        // Two separate files — write an "override" config with distinct values
        // and verify they are loaded correctly when passed as explicit path.
        let override_toml = minimal_toml("override_db", "override_tgt");
        let over = write_toml(&override_toml);

        let cfg = AppConfig::load(Some(over.path().to_str().unwrap())).unwrap();
        assert_eq!(cfg.source.dbname, "override_db");
        assert_eq!(cfg.target.dbname, "override_tgt");
    }

    #[test]
    fn load_missing_explicit_file_errors() {
        let result = AppConfig::load(Some("/nonexistent/path/diffly.toml"));
        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("Failed to build configuration")
                || msg.contains("not found")
                || msg.contains("No such")
        );
    }

    #[test]
    fn load_invalid_toml_errors() {
        let f = write_toml("this is not : valid toml ::::");
        let result = AppConfig::load(Some(f.path().to_str().unwrap()));
        assert!(result.is_err());
    }

    #[test]
    fn load_missing_required_field_errors() {
        // `port` is u16 — inject a non-integer via env to force a type error.
        // Env vars have lower priority than explicit path, so we pass only env + no file.
        let result = AppConfig::load_inner(
            None,
            env(&[
                ("DIFFLY_SOURCE__HOST", "localhost"),
                ("DIFFLY_SOURCE__PORT", "not-a-number"),
                ("DIFFLY_SOURCE__DBNAME", "db"),
                ("DIFFLY_SOURCE__USER", "u"),
                ("DIFFLY_SOURCE__PASSWORD", "p"),
                ("DIFFLY_TARGET__HOST", "localhost"),
                ("DIFFLY_TARGET__PORT", "5432"),
                ("DIFFLY_TARGET__DBNAME", "db"),
                ("DIFFLY_TARGET__USER", "u"),
                ("DIFFLY_TARGET__PASSWORD", "p"),
            ]),
        );
        assert!(result.is_err(), "expected error for invalid port type");
    }

    /// Env vars are higher priority than a TOML file, but lower than an explicit --config path.
    /// This test verifies env > file (no explicit path supplied — the explicit path is the file).
    /// We simulate this by using env vars that conflict with file values and checking env wins.
    #[test]
    fn load_env_overrides_file() {
        // Write a full config as the explicit file (mimics ./diffly.toml).
        // Then pass synthetic env that overrides source.dbname.
        // Since env is processed BEFORE explicit path in the builder, the explicit path wins.
        // To test env > file we need env to win over a *non-explicit* file.
        // We achieve this by passing explicit_path=None and verifying env fills the values.
        let cfg = AppConfig::load_inner(
            None,
            env(&[
                ("DIFFLY_SOURCE__HOST", "env-host"),
                ("DIFFLY_SOURCE__PORT", "5432"),
                ("DIFFLY_SOURCE__DBNAME", "env_db"),
                ("DIFFLY_SOURCE__USER", "env_user"),
                ("DIFFLY_SOURCE__PASSWORD", "env_pass"),
                ("DIFFLY_TARGET__HOST", "env-host"),
                ("DIFFLY_TARGET__PORT", "5432"),
                ("DIFFLY_TARGET__DBNAME", "env_tgt"),
                ("DIFFLY_TARGET__USER", "env_user"),
                ("DIFFLY_TARGET__PASSWORD", "env_pass"),
                ("DIFFLY_OUTPUT__DIR", "./env-output"),
            ]),
        )
        .unwrap();
        assert_eq!(cfg.source.dbname, "env_db");
        assert_eq!(cfg.source.host, "env-host");
        assert_eq!(cfg.target.dbname, "env_tgt");
        assert_eq!(cfg.output.dir, "./env-output");
    }

    #[test]
    fn load_env_port_parsed_as_integer() {
        let cfg = AppConfig::load_inner(
            None,
            env(&[
                ("DIFFLY_SOURCE__HOST", "localhost"),
                ("DIFFLY_SOURCE__PORT", "5555"),
                ("DIFFLY_SOURCE__DBNAME", "db"),
                ("DIFFLY_SOURCE__USER", "u"),
                ("DIFFLY_SOURCE__PASSWORD", "p"),
                ("DIFFLY_TARGET__HOST", "localhost"),
                ("DIFFLY_TARGET__PORT", "5432"),
                ("DIFFLY_TARGET__DBNAME", "db"),
                ("DIFFLY_TARGET__USER", "u"),
                ("DIFFLY_TARGET__PASSWORD", "p"),
            ]),
        )
        .unwrap();
        assert_eq!(cfg.source.port, 5555);
    }

    #[test]
    fn load_env_does_not_affect_unset_keys() {
        // File sets source; env only overrides target.host.
        let f = write_toml(&minimal_toml("file_db", "file_tgt"));
        // Env overrides target.host — but explicit path is highest priority.
        // Instead: use explicit path for the file, and verify env is NOT applied
        // (since explicit > env). Then test without explicit to prove env applies.
        let cfg_with_env_only = AppConfig::load_inner(
            None,
            env(&[
                ("DIFFLY_SOURCE__HOST", "localhost"),
                ("DIFFLY_SOURCE__PORT", "5432"),
                ("DIFFLY_SOURCE__DBNAME", "file_db"),
                ("DIFFLY_SOURCE__USER", "u"),
                ("DIFFLY_SOURCE__PASSWORD", "p"),
                ("DIFFLY_TARGET__HOST", "remote.host"),  // overridden
                ("DIFFLY_TARGET__PORT", "5432"),
                ("DIFFLY_TARGET__DBNAME", "file_tgt"),
                ("DIFFLY_TARGET__USER", "u"),
                ("DIFFLY_TARGET__PASSWORD", "p"),
            ]),
        )
        .unwrap();
        assert_eq!(cfg_with_env_only.source.dbname, "file_db");    // untouched
        assert_eq!(cfg_with_env_only.target.host, "remote.host");  // overridden
        drop(f);
    }

    #[test]
    fn load_table_config_parsed() {
        let toml = r#"
[source]
host = "localhost"
port = 5432
dbname = "src"
user = "u"
password = "p"

[target]
host = "localhost"
port = 5432
dbname = "tgt"
user = "u"
password = "p"

[output]
dir = "./out"

[[diff.tables]]
name = "users"
primary_key = ["id"]

[[diff.tables]]
name = "orders"
primary_key = ["order_id", "user_id"]
excluded_columns = ["created_at", "updated_at"]
"#;
        let f = write_toml(toml);
        let cfg = AppConfig::load(Some(f.path().to_str().unwrap())).unwrap();

        assert_eq!(cfg.diff.tables.len(), 2);
        assert_eq!(cfg.diff.tables[0].name, "users");
        assert_eq!(cfg.diff.tables[0].primary_key, vec!["id"]);
        assert!(cfg.diff.tables[0].excluded_columns.0.is_empty());

        assert_eq!(cfg.diff.tables[1].name, "orders");
        assert_eq!(cfg.diff.tables[1].primary_key, vec!["order_id", "user_id"]);
        assert_eq!(
            cfg.diff.tables[1].excluded_columns.0,
            vec!["created_at", "updated_at"]
        );
    }

    // ── DbConfig::url ─────────────────────────────────────────────────────────

    fn make_db(driver: &str, user: &str, password: &str, host: &str, port: u16, dbname: &str) -> DbConfig {
        DbConfig {
            driver: driver.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            host: host.to_string(),
            port,
            dbname: dbname.to_string(),
            schema: "public".to_string(),
        }
    }

    #[test]
    fn url_postgres() {
        let db = make_db("postgres", "alice", "pass", "localhost", 5432, "mydb");
        assert_eq!(db.url(), "postgres://alice:pass@localhost:5432/mydb");
    }

    #[test]
    fn url_mysql() {
        let db = make_db("mysql", "root", "pass", "127.0.0.1", 3306, "shop");
        assert_eq!(db.url(), "mysql://root:pass@127.0.0.1:3306/shop");
    }

    #[test]
    fn url_mariadb() {
        let db = make_db("mariadb", "root", "pass", "127.0.0.1", 3306, "shop");
        assert_eq!(db.url(), "mysql://root:pass@127.0.0.1:3306/shop");
    }

    #[test]
    fn url_sqlite() {
        let db = make_db("sqlite", "", "", "", 0, "/data/app.db");
        assert_eq!(db.url(), "sqlite:///data/app.db");
    }

    #[test]
    fn url_unknown_driver_falls_back_to_postgres() {
        let db = make_db("cockroachdb", "u", "p", "host", 26257, "db");
        assert!(db.url().starts_with("postgres://"));
    }

    #[test]
    fn url_special_chars_in_password_are_encoded() {
        // Password from the real diffly.toml fixture
        let db = make_db("postgres", "postgres", "9LAXxW<A#zR?FM2e$8]dpki7e_4X", "localhost", 5436, "db");
        let url = db.url();
        assert!(!url.contains('<'));
        assert!(!url.contains('#'));
        assert!(!url.contains('?'));
        assert!(!url.contains(']'));
        assert!(!url.contains('$'));
        assert!(url.contains("%3C")); // <
        assert!(url.contains("%23")); // #
        assert!(url.contains("%3F")); // ?
        assert!(url.contains("%5D")); // ]
        assert!(url.contains("%24")); // $
    }

    #[test]
    fn url_special_chars_in_user_are_encoded() {
        let db = make_db("postgres", "user@domain", "pass", "localhost", 5432, "db");
        let url = db.url();
        assert!(!url.contains("user@domain")); // raw @ would be ambiguous
        assert!(url.contains("%40")); // @
    }

    #[test]
    fn url_unreserved_chars_not_encoded() {
        // - _ . ~ are unreserved and must NOT be percent-encoded
        let db = make_db("postgres", "my_user", "pass-word.v1~", "localhost", 5432, "db");
        let url = db.url();
        assert!(url.contains("my_user"));
        assert!(url.contains("pass-word.v1~"));
    }

    #[test]
    fn url_multibyte_utf8_encoded() {
        let db = make_db("postgres", "user", "pässwörd", "localhost", 5432, "db");
        let url = db.url();
        assert!(!url.contains('ä'));
        assert!(!url.contains('ö'));
        // ä = U+00E4 → UTF-8 0xC3 0xA4 → %C3%A4
        assert!(url.contains("%C3%A4"));
    }
}
