use crate::domain::{changeset::Changeset, ports::OutputWriter};
use anyhow::Result;
use std::fs;

use self::{html::HtmlWriter, json::JsonWriter, sql::SqlWriter};

pub mod html;
pub mod json;
pub mod sql;

/// Register available writers - OCP: add new ones without touching main.rs
pub fn all_writers() -> Vec<Box<dyn OutputWriter>> {
    vec![
        Box::new(JsonWriter),
        Box::new(SqlWriter),
        Box::new(HtmlWriter),
    ]
}

pub fn writer_for(format: &str) -> Option<Box<dyn OutputWriter>> {
    match format {
        "json" => Some(Box::new(JsonWriter)),
        "sql" => Some(Box::new(SqlWriter)),
        "html" => Some(Box::new(HtmlWriter)),
        _ => None,
    }
}

/// Writes the changeset to disk via the chosen writer
pub fn write_to_file(writer: &dyn OutputWriter, changeset: &Changeset, dir: &str) -> Result<()> {
    // Ensure the output directory exists
    fs::create_dir_all(dir)?;

    let content = writer.format(changeset)?;
    let path = format!("{}/{}.{}", dir, changeset.changeset_id, writer.extension());
    fs::write(&path, &content)?;
    Ok(())
}
