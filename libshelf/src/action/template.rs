pub use crate::spec::{HandlebarsPartials, Tree};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::fsutil;

use super::error::FileMissingError;
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

#[derive(Debug, thiserror::Error)]
pub enum HandlebarsActionError {
    #[error("file missing")]
    FileMissing(#[from] FileMissingError),
    #[error("i/o error")]
    Io(#[from] io::Error),
    #[error("handlebars template error")]
    Template(#[from] handlebars::TemplateError),
    #[error("handlebars render error")]
    Render(#[from] handlebars::RenderError),
}

impl From<hbst::Error> for HandlebarsActionError {
    #[inline]
    fn from(err: hbst::Error) -> Self {
        match err {
            hbst::Error::Io(err) => err.into(),
            hbst::Error::Template(err) => err.into(),
            hbst::Error::Render(err) => err.into(),
        }
    }
}

impl Resolve for HandlebarsAction {
    type Error = HandlebarsActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'_>, Self::Error> {
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
                return Ok(Resolution::Skip(SkipReason::OptionalFileMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(Self::Error::FileMissing(FileMissingError {
                    path: src.clone(),
                }));
            }
            _ => {}
        };

        // Render contents.
        let contents = self::hbst::render(&src, &vars, &partials)?;

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

#[derive(Debug, thiserror::Error)]
pub enum LiquidActionError {
    #[error("file missing")]
    FileMissing(#[from] FileMissingError),
    #[error("i/o error")]
    Io(#[from] io::Error),
    #[error("liquid error")]
    Liquid(#[from] liquid::Error),
}

impl From<liquidt::Error> for LiquidActionError {
    #[inline]
    fn from(err: liquidt::Error) -> Self {
        match err {
            liquidt::Error::Io(err) => err.into(),
            liquidt::Error::Liquid(err) => err.into(),
        }
    }
}

impl Resolve for LiquidAction {
    type Error = LiquidActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'_>, Self::Error> {
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
                return Ok(Resolution::Skip(SkipReason::OptionalFileMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(Self::Error::FileMissing(FileMissingError {
                    path: src.clone(),
                }));
            }
            _ => {}
        };

        // Render contents.
        let contents = self::liquidt::render(src, vars)?;

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

mod hbst {
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

mod liquidt {
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
