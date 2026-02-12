use crate::domain::changeset::Changeset;
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
