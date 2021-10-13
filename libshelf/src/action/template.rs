use crate::fsutil;
pub use crate::spec::{HandlebarsPartials, Tree};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::SkipReason;
use super::{Resolution, Resolve, ResolveOpts, WriteAction, WriteActionError};

#[derive(Debug, Clone)]
pub struct HandlebarsAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub vars: Tree,

    pub optional: bool,
    pub partials: HandlebarsPartials,
}

pub type HandlebarsActionError = hbs::Error;

impl Resolve for HandlebarsAction {
    type Error = HandlebarsActionError;

    #[inline]
    fn resolve<'lua>(&self, opts: &ResolveOpts) -> Result<Resolution<'lua>, Self::Error> {
        let Self {
            src,
            dest,
            vars,
            optional,
            partials,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (optional, fsutil::exists(src)) {
            (true, false) => {
                return Ok(Resolution::Skip(SkipReason::OptionalMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(Self::Error::FileMissing { path: src.clone() });
            }
            _ => {}
        };

        // Render contents.
        let contents = self::hbs::render(&src, &vars, &partials)?;

        // Write the contents.
        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };

        let resolution = WriteActionError::unwrap(wa.resolve(opts));
        Ok(resolution)
    }
}

#[derive(Debug, Clone)]
pub struct LiquidAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub vars: Tree,

    pub optional: bool,
}

pub type LiquidActionError = liquid::Error;

impl Resolve for LiquidAction {
    type Error = LiquidActionError;

    #[inline]
    fn resolve<'lua>(&self, opts: &ResolveOpts) -> Result<Resolution<'lua>, Self::Error> {
        let Self {
            src,
            dest,
            vars,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (optional, fsutil::exists(src)) {
            (true, false) => {
                return Ok(Resolution::Skip(SkipReason::OptionalMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(Self::Error::FileMissing { path: src.clone() });
            }
            _ => {}
        };

        // Render contents.
        let contents = self::liquid::render(src.abs(), &vars)?;

        // Write contents.
        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        let resolution = WriteActionError::unwrap(wa.resolve(opts));
        Ok(resolution)
    }
}

#[inline]
fn read_template<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

mod hbs {
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

mod liquid {
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
