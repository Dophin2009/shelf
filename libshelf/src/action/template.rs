use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::fse;

use super::error::FileMissingError;
use super::write::{Res as WriteActionRes, WriteAction};
use super::Resolve;

// Re-export action types.
pub use self::{hbs::HandlebarsAction, liquid::LiquidAction};
// Re-export Res types.
pub use super::write::Op;
// Re-export shared Object type.
pub use super::object::Object;

#[derive(Debug, Clone)]
pub enum Res {
    Normal(Vec<Op>),
    /// The existing destination file's contents will be overwritten.
    OverwriteContents(Vec<Op>),
    /// The existing destination file will be replaced.
    OverwriteFile(Vec<Op>),
    /// The action is skipped.
    Skip(Skip),
}

impl Res {
    #[inline]
    fn from_write_action_res(res: WriteActionRes) -> Self {
        use super::write::Skip as WriteActionSkip;

        match res {
            WriteActionRes::Normal(ops) => Self::Normal(ops),
            WriteActionRes::OverwriteContents(ops) => Self::OverwriteContents(ops),
            WriteActionRes::OverwriteFile(ops) => Self::OverwriteFile(ops),
            WriteActionRes::Skip(skip) => Self::Skip(match skip {
                WriteActionSkip::DestExists => Skip::DestExists,
            }),
        }
    }
}

/// Reason for skipping [`HandlebarsAction`] or [`LiquidAction`].
#[derive(Debug, Clone)]
pub enum Skip {
    /// `src` and `dest` are the same path.
    SameSrcDest,
    /// Optional `src` does not exist.
    OptMissing,
    /// Destination link already exists.
    DestExists,
}

pub mod hbs {
    use std::collections::HashMap;
    use std::io;
    use std::path::{Path, PathBuf};

    use handlebars::Handlebars;
    use serde::Serialize;

    use super::{FileMissingError, Object, Res, Resolve};

    // Re-export handlebars error types.
    pub use handlebars::{RenderError, TemplateError};

    pub type HandlebarsPartials = HashMap<String, PathBuf>;

    #[derive(Debug, Clone)]
    pub struct HandlebarsAction {
        pub src: PathBuf,
        pub dest: PathBuf,
        pub vars: Object,

        pub optional: bool,
        pub partials: HandlebarsPartials,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("src missing")]
        SrcMissing,
        #[error("i/o error")]
        Io(#[from] io::Error),
        #[error("handlebars template error")]
        Template(#[from] TemplateError),
        #[error("handlebars render error")]
        Render(#[from] RenderError),
    }

    impl Resolve for HandlebarsAction {
        type Output = Result<Res, Error>;

        #[inline]
        fn resolve(&self) -> Self::Output {
            let Self {
                src,
                dest,
                vars,
                optional,
                partials,
            } = self;

            super::resolve_impl(src, dest, vars, optional, |src, _dest, vars| {
                render(src, vars, partials)
            })
            .and_then(|res| res.ok_or(Error::SrcMissing))
        }
    }

    #[inline]
    fn render<P: AsRef<Path>, S: Serialize>(
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
    use std::path::{Path, PathBuf};

    use liquid::ParserBuilder;
    use serde::Serialize;

    use super::{FileMissingError, Object, Res, Resolve};

    // Re-export liquid error type.
    pub use liquid::Error as LiquidError;

    #[derive(Debug, Clone)]
    pub struct LiquidAction {
        pub src: PathBuf,
        pub dest: PathBuf,
        pub vars: Object,

        pub optional: bool,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("src missing")]
        SrcMissing,
        #[error("i/o error")]
        Io(#[from] io::Error),
        #[error("liquid error")]
        Liquid(#[from] LiquidError),
    }

    impl Resolve for LiquidAction {
        type Output = Result<Res, Error>;

        #[inline]
        fn resolve(&self) -> Self::Output {
            let Self {
                src,
                dest,
                vars,
                optional,
            } = self;

            super::resolve_impl(src, dest, vars, optional, |src, _dest, vars| {
                render(src, vars)
            })
            .and_then(|res| res.ok_or(Error::SrcMissing))
        }
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

#[inline]
fn resolve_impl<E, RF>(
    src: &Path,
    dest: &Path,
    vars: &Object,
    optional: &bool,
    render: RF,
) -> Result<Option<Res>, E>
where
    RF: Fn(&Path, &Path, &Object) -> Result<String, E>,
{
    if src == dest {
        return Ok(Some(Res::Skip(Skip::SameSrcDest)));
    }

    match (optional, fse::symlink_exists(src)) {
        // `src` is optional and does not exist, skip.
        (true, false) => Ok(Some(Res::Skip(Skip::OptMissing))),
        // `src` is not optional but does not exist, error.
        (false, false) => Ok(None),
        // Otherwise, `src` exists.
        _ => {
            // Render contents.
            let contents = render(src, dest, vars)?;

            // Write the contents.
            let wa = WriteAction {
                dest: dest.to_path_buf(),
                contents: contents.into_bytes(),
            };
            let res = wa.resolve();

            Ok(Some(Res::from_write_action_res(res)))
        }
    }
}

#[inline]
fn read_template<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}
