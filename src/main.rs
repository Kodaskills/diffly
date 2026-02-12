use anyhow::Result;
use chrono::Local;
use clap::Parser;
use diffly::presentation::cli_summary::print_summary;
use diffly::presentation::writers::{all_writers, write_to_file, writer_for};
use diffly::AppConfig;
use std::path::Path;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser, Debug)]
#[command(
    name = "diffly",
    about = "Diffly — Quickly compare your SQL data with clarity and style."
)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[arg(long)]
    dry_run: bool,

    #[arg(short, long, default_value = "all")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "diffly=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let cfg = AppConfig::load(&cli.config)?;
    let changeset = diffly::run(&cfg).await?;

    if cli.dry_run {
        print_summary(&changeset);
        return Ok(());
    }

    // --- generate subdirectory per changeset ---
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let subdir_path = changeset.driver.to_string();
    let subdir_name = format!("{}_{}", timestamp, changeset.changeset_id);
    let output_subdir = Path::new(&cfg.output.dir)
        .join(&subdir_path)
        .join(&subdir_name);

    // create the directory and all parents if needed
    std::fs::create_dir_all(&output_subdir)?;

    match cli.format.as_str() {
        "all" => {
            for writer in all_writers() {
                write_to_file(&*writer, &changeset, output_subdir.to_str().unwrap())?;
            }
        }
        fmt => {
            let writer =
                writer_for(fmt).ok_or_else(|| anyhow::anyhow!("Unknown format: {}", fmt))?;
            write_to_file(&*writer, &changeset, output_subdir.to_str().unwrap())?;
        }
    }

    println!("Changeset written to {}", output_subdir.display());

    Ok(())
}
