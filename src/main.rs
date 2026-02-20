use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use diffly::presentation::cli_summary::{print_conflicts, print_perf_summary, print_summary};
use diffly::presentation::writers::{all_writers, write_to_file, writer_for};
use diffly::{AppConfig, Fingerprint, LogLevel, RowMap};
use std::collections::BTreeMap;
use std::path::Path;

// ─── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "diffly",
    about = "Diffly — Quickly compare your SQL data with clarity and style.",
    version
)]
struct Cli {
    /// Path to a TOML config file. Overrides ./diffly.toml and ~/.config/diffly/diffly.toml.
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Enable debug-level tracing (overrides RUST_LOG).
    #[arg(long, global = true, conflicts_with = "quiet")]
    verbose: bool,

    /// Suppress all output except errors. Useful in CI / scripting.
    #[arg(long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compute a 2-way diff (source → target) and write output files.
    Diff {
        /// Print a summary to stdout without writing any files.
        #[arg(long)]
        dry_run: bool,

        /// Output format: json | sql | html | all (default: all).
        #[arg(short, long, default_value = "all")]
        format: String,
    },

    /// Capture a point-in-time snapshot of the target (target) DB.
    ///
    /// Writes two files to <config output dir/snapshots>:
    ///   snapshot.json      — all target rows per table
    ///   fingerprints.json  — per-table SHA-256 fingerprints
    ///
    /// Call this at target-clone time.
    /// Use with `check-conflicts --snapshot`
    /// when you are ready to check for conflicts.
    Snapshot {},

    /// Diff + 3-way conflict detection against a stored snapshot.
    ///
    /// Reads snapshot.json and fingerprints.json from <snapshot>,
    /// runs the diff, and checks for concurrent target changes.
    /// Exits with code 2 if conflicts are detected.
    CheckConflicts {
        /// Directory containing snapshot.json and fingerprints.json
        /// (produced by `diffly snapshot`).
        #[arg(short, long)]
        snapshot: String,

        /// Print a summary to stdout without writing any files.
        #[arg(long)]
        dry_run: bool,

        /// Output format: json | sql | html | all (default: all).
        #[arg(short, long, default_value = "all")]
        format: String,
    },
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = if cli.verbose {
        LogLevel::Debug
    } else if cli.quiet {
        LogLevel::Error
    } else {
        LogLevel::Info
    };

    diffly::init_tracing(level);

    let cfg = AppConfig::load(cli.config.as_deref())?;
    let quiet = cli.quiet;

    match cli.command {
        Command::Diff { dry_run, format } => cmd_diff(&cfg, dry_run, &format, quiet).await,
        Command::Snapshot {} => cmd_snapshot(&cfg, quiet).await,
        Command::CheckConflicts {
            snapshot,
            dry_run,
            format,
        } => cmd_check_conflicts(&cfg, &snapshot, dry_run, &format, quiet).await,
    }
}

// ─── Subcommand handlers ──────────────────────────────────────────────────────

/// `diffly diff` — 2-way diff only.
async fn cmd_diff(cfg: &AppConfig, dry_run: bool, format: &str, quiet: bool) -> Result<()> {
    let (changeset, perf) = diffly::run_with_timing(cfg).await?;

    if !quiet {
        print_summary(&changeset);
        print_perf_summary(&perf);
    }

    if dry_run {
        return Ok(());
    }

    write_changeset(cfg, &changeset, format)
}

/// `diffly snapshot` — capture target DB state.
async fn cmd_snapshot(cfg: &AppConfig, quiet: bool) -> Result<()> {
    if !quiet {
        println!("Capturing snapshot of target DB ({})…", cfg.target.schema);
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let subdir_name = format!("{}_{}", "snapshot", timestamp);
    let output_subdir = Path::new(&cfg.output.dir)
        .join(&cfg.target.driver)
        .join(&subdir_name);

    let (raw, perf) = diffly::snapshot_with_timing(cfg).await?;

    // Compute per-table fingerprints from the captured rows.
    let fps: BTreeMap<String, Fingerprint> = raw
        .iter()
        .map(|(table, rows)| (table.clone(), diffly::fingerprint(rows)))
        .collect();

    std::fs::create_dir_all(&output_subdir).with_context(|| {
        format!(
            "Failed to create snapshot directory: {}",
            output_subdir.to_str().unwrap()
        )
    })?;

    let snapshot_path = Path::new(output_subdir.to_str().unwrap()).join("snapshot.json");
    let fp_path = Path::new(output_subdir.to_str().unwrap()).join("fingerprints.json");

    std::fs::write(&snapshot_path, serde_json::to_string_pretty(&raw)?)
        .with_context(|| format!("Failed to write {}", snapshot_path.display()))?;
    std::fs::write(&fp_path, serde_json::to_string_pretty(&fps)?)
        .with_context(|| format!("Failed to write {}", fp_path.display()))?;

    if !quiet {
        print_perf_summary(&perf);
        println!("  snapshot     → {}", snapshot_path.display());
        println!("  fingerprints → {}", fp_path.display());
        println!("Done. {} table(s) captured.", raw.len());
    }

    Ok(())
}

/// `diffly check-conflicts` — diff + 3-way conflict detection.
async fn cmd_check_conflicts(
    cfg: &AppConfig,
    snapshot_dir: &str,
    dry_run: bool,
    format: &str,
    quiet: bool,
) -> Result<()> {
    let snapshot_path = Path::new(snapshot_dir).join("snapshot.json");
    let fp_path = Path::new(snapshot_dir).join("fingerprints.json");

    let raw: BTreeMap<String, Vec<RowMap>> = serde_json::from_str(
        &std::fs::read_to_string(&snapshot_path)
            .with_context(|| format!("Cannot read {}", snapshot_path.display()))?,
    )
    .with_context(|| format!("Failed to parse {}", snapshot_path.display()))?;

    let stored_fps: BTreeMap<String, Fingerprint> = serde_json::from_str(
        &std::fs::read_to_string(&fp_path)
            .with_context(|| format!("Cannot read {}", fp_path.display()))?,
    )
    .with_context(|| format!("Failed to parse {}", fp_path.display()))?;

    // Fetch current target rows with timing.
    let (current_rows, snapshot_perf) = diffly::snapshot_with_timing(cfg).await?;

    // Run diff + conflict detection with timing.
    let (changeset, diff_perf) = diffly::run_with_timing(cfg).await?;

    let base = diffly::snapshot_provider(raw);
    let pk_cols_by_table: std::collections::BTreeMap<String, Vec<diffly::ColumnName>> = cfg
        .diff
        .tables
        .iter()
        .map(|t| {
            let cols = t
                .primary_key
                .iter()
                .map(|pk| diffly::ColumnName(pk.clone()))
                .collect();
            (t.name.clone(), cols)
        })
        .collect();
    let result = diffly::application::conflict::ConflictService::new().check(
        changeset,
        &base,
        &stored_fps,
        &current_rows,
        &pk_cols_by_table,
    );

    let changeset = result.changeset();

    if !quiet {
        print_summary(changeset);
        print_perf_summary(&snapshot_perf);
        print_perf_summary(&diff_perf);
    }

    // Conflicts are always reported (even in quiet mode) — they are
    // actionable errors, not informational output.
    let has_conflicts = print_conflicts(result.conflicts());

    if has_conflicts {
        // Exit code 2 = conflicts (distinct from error exit 1).
        std::process::exit(2);
    }

    if dry_run {
        return Ok(());
    }

    write_changeset(cfg, changeset, format)
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

fn write_changeset(cfg: &AppConfig, changeset: &diffly::Changeset, format: &str) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let subdir_name = format!("{}_{}", timestamp, changeset.changeset_id);
    let output_subdir = Path::new(&cfg.output.dir)
        .join(&changeset.driver)
        .join(&subdir_name);

    std::fs::create_dir_all(&output_subdir)?;

    match format {
        "all" => {
            for writer in all_writers() {
                write_to_file(&*writer, changeset, output_subdir.to_str().unwrap())?;
            }
        }
        fmt => {
            let writer =
                writer_for(fmt).ok_or_else(|| anyhow::anyhow!("Unknown format: {}", fmt))?;
            write_to_file(&*writer, changeset, output_subdir.to_str().unwrap())?;
        }
    }

    println!("Changeset written to {}", output_subdir.display());
    Ok(())
}
