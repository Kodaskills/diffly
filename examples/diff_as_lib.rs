//! # Diffly — library usage example
//!
//! Shows three common patterns for consuming Diffly as a Rust library:
//!
//! 1. **From a config file** — simplest, mirrors the CLI
//! 2. **Programmatic config** — build `AppConfig` in code, no TOML file needed
//! 3. **Inspect the changeset** — traverse the diff result for custom logic
//!
//! Run with a config file:
//!   cargo run --example diff_as_lib -- diffly.toml
//!
//! Run with the built-in programmatic config (needs a local PostgreSQL):
//!   cargo run --example diff_as_lib

use std::collections::BTreeMap;

use anyhow::Result;
use diffly::{
    presentation::writers::{all_writers, write_to_file, writer_for},
    AppConfig, Changeset, DbConfig, DiffConfig, ExcludedColumns, OutputConfig, TableConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some(path) => from_config_file(path).await,
        None => programmatic_config().await,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern 1 — load config from a TOML file (same as the CLI does internally)
// ─────────────────────────────────────────────────────────────────────────────
async fn from_config_file(path: &str) -> Result<()> {
    println!("=== Pattern 1: from config file ({path}) ===\n");

    let cfg = AppConfig::load(path)?;
    let changeset = diffly::run(&cfg).await?;

    // Write all three output formats (JSON / SQL / HTML)
    for writer in all_writers() {
        write_to_file(&*writer, &changeset, &cfg.output.dir)?;
        println!(
            "Written: {}/{}.{}",
            cfg.output.dir,
            changeset.changeset_id,
            writer.extension()
        );
    }

    print_summary(&changeset);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern 2 — build AppConfig entirely in code, no TOML file required.
// Useful when config comes from env vars, a CLI flag, a database row, etc.
// ─────────────────────────────────────────────────────────────────────────────
async fn programmatic_config() -> Result<()> {
    println!("=== Pattern 2: programmatic config ===\n");

    let db = |schema: &str| DbConfig {
        driver: "postgres".into(),
        host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".into()),
        port: 5432,
        dbname: "diffly".into(),
        user: "diffly".into(),
        password: "diffly".into(),
        schema: schema.into(),
    };

    let cfg = AppConfig {
        source: db("source"),
        target: db("target"),
        diff: DiffConfig {
            tables: vec![
                TableConfig {
                    name: "pricing_rules".into(),
                    primary_key: vec!["id".into()],
                    excluded_columns: ExcludedColumns(vec![
                        "created_at".into(),
                        "updated_at".into(),
                    ]),
                },
                TableConfig {
                    name: "discount_tiers".into(),
                    primary_key: vec!["id".into()],
                    excluded_columns: ExcludedColumns::default(),
                },
                TableConfig {
                    name: "tax_rules".into(),
                    primary_key: vec!["region_code".into(), "product_category".into()],
                    excluded_columns: ExcludedColumns::default(),
                },
            ],
        },
        output: OutputConfig {
            dir: "./output".into(),
        },
    };

    let changeset = diffly::run(&cfg).await?;

    // Write only the SQL migration file
    let sql_writer = writer_for("sql").expect("sql writer always available");
    write_to_file(&*sql_writer, &changeset, &cfg.output.dir)?;
    println!(
        "SQL written: {}/{}.sql\n",
        cfg.output.dir, changeset.changeset_id
    );

    // Hand off to pattern 3
    inspect_changeset(&changeset);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern 3 — inspect the Changeset directly for custom logic.
// The Changeset is plain serialisable Rust data — no magic, no callbacks.
// ─────────────────────────────────────────────────────────────────────────────
fn inspect_changeset(changeset: &Changeset) {
    println!("=== Pattern 3: inspecting the changeset ===\n");
    println!("id      : {}", changeset.changeset_id);
    println!("source  : {}", changeset.source_schema);
    println!("target  : {}", changeset.target_schema);
    println!("driver  : {}", changeset.driver);
    println!();

    for table in &changeset.tables {
        if table.is_empty() {
            continue;
        }

        println!("━━ {} ━━", table.table_name);

        for ins in &table.inserts {
            println!("  + INSERT  {}", fmt_pk(&ins.pk));
        }

        for upd in &table.updates {
            let pk = fmt_pk(&upd.pk);
            for col in &upd.changed_columns {
                println!(
                    "  ~ UPDATE  {}  {}: {} → {}",
                    pk, col.column, col.before, col.after
                );
            }
        }

        for del in &table.deletes {
            println!("  - DELETE  {}", fmt_pk(&del.pk));
        }

        println!();
    }

    // Example: abort a deployment pipeline when unexpected deletes are detected
    if changeset.summary.total_deletes > 0 {
        eprintln!(
            "⚠  {} delete(s) detected — review before applying to production.",
            changeset.summary.total_deletes,
        );
    }

    // Example: serialise to JSON and send to a webhook / write to a log
    let json = serde_json::to_string_pretty(changeset).expect("Changeset is always serialisable");
    println!("Full changeset: {} bytes of JSON", json.len());

    print_summary(changeset);
}

fn fmt_pk(pk: &BTreeMap<String, serde_json::Value>) -> String {
    pk.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_summary(changeset: &Changeset) {
    println!("\n── summary ──────────────────────");
    println!("  inserts : {}", changeset.summary.total_inserts);
    println!("  updates : {}", changeset.summary.total_updates);
    println!("  deletes : {}", changeset.summary.total_deletes);
    println!("  tables  : {}", changeset.summary.tables_affected);
}
