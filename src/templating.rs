use anyhow::Result;
use serde::Serialize;

pub trait TemplateEngine {
    fn render<S: Serialize>(&self, value: &S) -> Result<String>;
}

pub mod hbs {
    use std::collections::HashMap;
    use std::fs;
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
        let template_str =
            fs::read_to_string(template).with_context(|| "Couldn't read template source")?;

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
