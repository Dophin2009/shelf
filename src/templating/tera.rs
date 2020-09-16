use anyhow::Result;
use serde::Serialize;
use tera::{Context, Tera};

use std::path::Path;

pub fn render<P: AsRef<Path>, S: Serialize>(path: P, context: &S) -> Result<String> {
    let path_str = &path.as_ref().to_string_lossy();
    let templater = Tera::new(path_str)?;
    let rendered = templater.render(path_str, &Context::from_serialize(&context)?)?;
    Ok(rendered)
}
