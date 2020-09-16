use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Templater;

impl Templater {
    pub fn render<T: Into<gtmpl::Value>>(template: &str, context: T) -> Result<String> {
        let rendered = gtmpl::template(template, context)
            .map_err(|err| anyhow!("Failed to render template: {}", err))?;
        Ok(rendered)
    }
}
