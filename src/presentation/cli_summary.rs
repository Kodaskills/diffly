use crate::application::monitoring::PerfReport;
use crate::domain::changeset::Changeset;
use crate::domain::conflict::ConflictReport;
use colored::*;
use tabled::settings::{object::Columns, Alignment, Modify, Style};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct TableRow {
    table: String,
    inserts: String,
    updates: String,
    deletes: String,
}

#[derive(Tabled)]
struct SummaryRow {
    metric: String,
    value: String,
}

pub fn print_summary(changeset: &Changeset) {
    println!();

    println!("{}", "DIFFLY DIFF SUMMARY".bold().cyan());
    println!(
        "{} → {}",
        changeset.source_schema.blue(),
        changeset.target_schema.green()
    );
    println!("Changeset: {}", changeset.changeset_id.bright_yellow());
    println!();

    if changeset.summary.total_changes == 0 {
        println!("{}", "No changes detected.".italic());
        return;
    }

    let rows: Vec<TableRow> = changeset
        .tables
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| TableRow {
            table: t.table_name.bold().to_string(),
            inserts: t.inserts.len().to_string().green().to_string(),
            updates: t.updates.len().to_string().yellow().to_string(),
            deletes: t.deletes.len().to_string().red().to_string(),
        })
        .collect();

    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::new(1..=5)).with(Alignment::right()))
        .to_string();
    println!("{table}");

    let s = &changeset.summary;
    let summary_rows = vec![
        SummaryRow {
            metric: "Total inserts".into(),
            value: s.total_inserts.to_string().green().to_string(),
        },
        SummaryRow {
            metric: "Total updates".into(),
            value: s.total_updates.to_string().yellow().to_string(),
        },
        SummaryRow {
            metric: "Total deletes".into(),
            value: s.total_deletes.to_string().red().to_string(),
        },
        SummaryRow {
            metric: "Total changes".into(),
            value: s.total_changes.to_string().bold().to_string(),
        },
    ];

    let summary_table = Table::new(summary_rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::new(1..=1)).with(Alignment::right()))
        .to_string();

    println!();
    println!("{summary_table}");
    println!();
}

// ─── Conflict summary ─────────────────────────────────────────────────────────

#[derive(Tabled)]
struct ConflictRow {
    table: String,
    pk: String,
    column: String,
    base: String,
    yours: String,
    theirs: String,
}

/// Print a coloured table of all detected conflicts to stdout.
///
/// Returns `true` if there are conflicts (so the caller can exit non-zero).
pub fn print_conflicts(conflicts: &[ConflictReport]) -> bool {
    if conflicts.is_empty() {
        println!("{}", "✓ No conflicts — changeset is clean.".bold().green());
        return false;
    }

    println!();
    println!("{}", "CONFLICTS DETECTED".bold().red());
    println!(
        "{} conflict(s) must be resolved before deploying.",
        conflicts.len().to_string().bold()
    );
    println!();

    let rows: Vec<ConflictRow> = conflicts
        .iter()
        .map(|c| {
            let pk_str =
                c.pk.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");
            ConflictRow {
                table: c.table_name.bold().to_string(),
                pk: pk_str,
                column: c.column.yellow().to_string(),
                base: c.base_value.to_string().dimmed().to_string(),
                yours: c.source_value.to_string().cyan().to_string(),
                theirs: c.target_value.to_string().red().to_string(),
            }
        })
        .collect();

    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::new(0..=0)).with(Alignment::left()))
        .to_string();

    println!("{table}");
    println!();
    println!("  {}  base value at clone time", "base  →".dimmed());
    println!("  {}  your source change", "yours →".cyan());
    println!("  {}  concurrent target change", "theirs→".red());
    println!();

    true
}

// ─── Performance summary ──────────────────────────────────────────────────────

#[derive(Tabled)]
struct PerfRow {
    operation: String,
    table: String,
    #[tabled(rename = "rows")]
    rows: String,
    #[tabled(rename = "time (ms)")]
    duration_ms: String,
}

/// Print a performance timing table to stdout.
pub fn print_perf_summary(report: &PerfReport) {
    if report.timings.is_empty() {
        return;
    }

    println!("{}", "PERFORMANCE".bold().cyan());

    let rows: Vec<PerfRow> = report
        .timings
        .iter()
        .map(|t| PerfRow {
            operation: t.operation.dimmed().to_string(),
            table: t.table.bold().to_string(),
            rows: t.rows.to_string(),
            duration_ms: format_duration(t.duration_ms),
        })
        .collect();

    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::new(2..=3)).with(Alignment::right()))
        .to_string();

    println!("{table}");

    println!(
        "  Total: {} row(s) fetched  ·  {} ms elapsed",
        report.total_rows_fetched().to_string().bold(),
        format_duration(report.total_ms()),
    );
    println!();
}

fn format_duration(ms: u128) -> String {
    if ms >= 1_000 {
        format!("{:.1}s", ms as f64 / 1_000.0).yellow().to_string()
    } else if ms >= 100 {
        ms.to_string().yellow().to_string()
    } else {
        ms.to_string().green().to_string()
    }
}
