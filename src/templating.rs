use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

pub trait TemplateEngine {
    fn render<S: Serialize>(&self, value: &S) -> Result<String>;
}

#[inline]
fn read_template<P: AsRef<Path>>(path: P) -> Result<String> {
    fs::read_to_string(path).with_context(|| "Couldn't read template source")
}

pub mod hbs {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result};
    use handlebars::Handlebars;
    use serde::Serialize;

    #[inline]
    pub fn render<P: AsRef<Path>, S: Serialize>(
        template: P,
        ctx: &S,
        partials: &HashMap<String, PathBuf>,
    ) -> Result<String> {
        let template_str = super::read_template(template)?;

        let mut reg = Handlebars::new();
        partials
            .iter()
            .map(|(name, path)| {
                reg.register_template_file(name, &path)
                    .with_context(|| "Couldn't register partial template")
            })
            .collect::<Result<Vec<_>>>()?;

        reg.render_template(&template_str, ctx)
            .with_context(|| "Couldn't render a handlebars template")
    }
}

pub mod liquid {
    use std::path::Path;

    use anyhow::{Context, Result};
    use liquid::ParserBuilder;
    use serde::Serialize;

    #[inline]
    pub fn render<P: AsRef<Path>, S: Serialize>(template: P, ctx: &S) -> Result<String> {
        let template_str = super::read_template(template)?;
        // FIXME error context
        let parser = ParserBuilder::with_stdlib().build()?.parse(&template_str)?;
        let object = liquid::to_object(ctx)?;

        parser
            .render(&object)
            .with_context(|| "Couldn't render a liquid template")
    }
}
