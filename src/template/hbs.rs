use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;

pub fn render<S: Serialize>(
    template: &str,
    context: &S,
    partials: &HashMap<String, PathBuf>,
) -> Result<String> {
    let mut reg = Handlebars::new();
    for (name, path) in partials {
        reg.register_template_file(name, &path)?;
    }

    let rendered = reg
        .render_template(template, context)
        .with_context(|| "Failed to render template")?;
    Ok(rendered)
}
