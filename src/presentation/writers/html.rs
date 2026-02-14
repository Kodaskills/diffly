use anyhow::Result;
use sailfish::TemplateOnce;

use crate::domain::{changeset::Changeset, ports::OutputWriter};

#[derive(TemplateOnce)]
#[template(path = "html/changeset.stpl")] // base dir declared inside sailfish.toml
struct ChangesetTemplate<'a> {
    changeset: &'a Changeset,
}

pub struct HtmlWriter;

impl OutputWriter for HtmlWriter {
    fn format(&self, changeset: &Changeset) -> Result<String> {
        Ok(ChangesetTemplate { changeset }.render_once()?)
    }

    fn extension(&self) -> &'static str {
        "html"
    }
}
