use anyhow::Result;
use serde::Serialize;
use tera::{Context, Tera};

pub fn render<S: Serialize>(template: &str, context: &S) -> Result<String> {
    let context = Context::from_serialize(&context)?;
    let rendered = Tera::one_off(template, &context, false)?;
    Ok(rendered)
}
