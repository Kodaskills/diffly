use anyhow::Result;

use crate::domain::{changeset::Changeset, ports::OutputWriter};

pub struct JsonWriter;

impl OutputWriter for JsonWriter {
    fn format(&self, cs: &Changeset) -> Result<String> {
        Ok(serde_json::to_string_pretty(cs)?)
    }

    fn extension(&self) -> &'static str {
        "json"
    }
}
