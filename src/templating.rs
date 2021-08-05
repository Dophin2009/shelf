use std::fs;
use std::io;
use std::path::Path;

#[inline]
fn read_template<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

pub mod hbs {
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    use handlebars::Handlebars;
    use serde::Serialize;

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("i/o error")]
        Io(#[from] io::Error),
        #[error("couldn't parse a handlebars template")]
        Template(#[from] handlebars::TemplateError),
        #[error("couldn't render a handlebars template")]
        Render(#[from] handlebars::RenderError),
    }

    #[inline]
    pub fn render<P: AsRef<Path>, S: Serialize>(
        template: P,
        ctx: &S,
        partials: &HashMap<String, PathBuf>,
    ) -> Result<String, Error> {
        let template_str = super::read_template(template)?;

        let mut reg = Handlebars::new();
        partials
            .iter()
            .map(|(name, path)| reg.register_template_file(name, &path))
            .collect::<Result<Vec<_>, _>>()?;

        let res = reg.render_template(&template_str, ctx)?;
        Ok(res)
    }
}

pub mod liquid {
    use std::io;
    use std::path::Path;

    use liquid::ParserBuilder;
    use serde::Serialize;

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("i/o error")]
        Io(#[from] io::Error),
        #[error("liquid error")]
        Liquid(#[from] liquid::Error),
    }

    #[inline]
    pub fn render<P: AsRef<Path>, S: Serialize>(template: P, ctx: &S) -> Result<String, Error> {
        let template_str = super::read_template(template)?;

        // FIXME error context
        let parser = ParserBuilder::with_stdlib().build()?.parse(&template_str)?;
        let object = liquid::to_object(ctx)?;

        let res = parser.render(&object)?;
        Ok(res)
    }
}
