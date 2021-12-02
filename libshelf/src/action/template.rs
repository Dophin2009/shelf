use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::FileMissingError;
use super::write::WriteAction;
use super::Resolve;

// Re-export action types.
pub use self::{hbs::HandlebarsAction, liquid::LiquidAction};
// Re-export shared Res type.
pub use super::write::Res;
// Re-export shared Object type.
pub use crate::object::Object;

/// Reason for skipping [`HandlebarsAction`] or [`LiquidAction`].
#[derive(Debug, Clone)]
pub enum Skip {
    /// `src` and `dest` are the same path.
    SameSrcDest(PathBuf),
    /// Optional `src` does not exist.
    OptMissing(PathBuf),
    /// Destination link already exists.
    DestExists(PathBuf),
}

pub mod hbs {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use handlebars::Handlebars;
    use serde::Serialize;

    use crate::fsutil;

    use super::{FileMissingError, Res, Resolve, Skip, WriteAction};

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
        #[error("file missing")]
        FileMissing(#[from] FileMissingError),
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

            match (optional, fsutil::exists(src)) {
                // `src` is optional and does not exist, skip.
                (true, false) => {
                    Ok(Res::Skip(Skip::OptMissing(src.clone())));
                }
                // `src` is not optional but does not exist, error.
                (false, false) => {
                    Err(Error::FileMissing(FileMissingError { path: src.clone() }));
                }
                // Otherwise, `src` exists.
                _ => {
                    // Render contents.
                    let contents = render(src, vars, partials)?;

                    // Write the contents.
                    let wa = WriteAction {
                        dest: dest.clone(),
                        contents,
                    };
                    Ok(wa.resolve())
                }
            }
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

mod liquid {
    use std::io;
    use std::path::{Path, PathBuf};

    use liquid::ParserBuilder;
    use serde::Serialize;

    use super::{FileMissingError, Res, Resolve, Skip, WriteAction};

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
        #[error("file missing")]
        FileMissing(#[from] FileMissingError),
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

            match (optional, fsutil::exists(src)) {
                // `src` is optional and does not exist, skip.
                (true, false) => {
                    Ok(Res::Skip(Skip::OptMissing(src.clone())));
                }
                // `src` is not optional but does not exist, error.
                (false, false) => {
                    Err(Error::FileMissing(FileMissingError { path: src.clone() }));
                }
                // Otherwise, `src` exists.
                _ => {
                    // Render contents.
                    let contents = render(src, vars)?;

                    // Write contents.
                    let wa = WriteAction {
                        dest: dest.clone(),
                        contents,
                    };
                    Ok(wa.resolve())
                }
            }
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
fn read_template<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}
