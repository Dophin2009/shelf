use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use console::style;
use glob::{GlobError, PatternError};
use mlua::Function;

use crate::format::{Indexed, Sublevel};
use crate::spec::{HandlebarsPartials, Patterns};
use crate::templating;
use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolvable {
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError>;
}

pub enum Resolution {
    Done,
    Skipped,
}

// FIXME better grouping
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("couldn't find file")]
    Io(#[from] io::Error),
    #[error("couldn't glob with a pattern")]
    Glob(#[from] GlobError),
    #[error("couldn't parse a pattern")]
    Pattern(#[from] PatternError),
    #[error("handlebars error")]
    Hbs(#[from] templating::hbs::Error),
    #[error("liquid error")]
    Liquid(#[from] templating::liquid::Error),
    #[error("yaml error")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json error")]
    Json(#[from] serde_json::Error),
    #[error("toml error")]
    Toml(#[from] toml::ser::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub enum Action<'lua> {
    Link(LinkAction),
    Write(WriteAction),
    Tree(TreeAction),
    Handlebars(HandlebarsAction),
    Liquid(LiquidAction),
    Yaml(YamlAction),
    Toml(TomlAction),
    Json(JsonAction),
    Command(CommandAction),
    Function(FunctionAction<'lua>),
}

impl<'a> Resolvable for Action<'a> {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        match self {
            Self::Link(a) => a.resolve(opts),
            Self::Write(a) => a.resolve(opts),
            Self::Tree(a) => a.resolve(opts),
            Self::Handlebars(a) => a.resolve(opts),
            Self::Liquid(a) => a.resolve(opts),
            Self::Yaml(a) => a.resolve(opts),
            Self::Toml(a) => a.resolve(opts),
            Self::Json(a) => a.resolve(opts),
            Self::Command(a) => a.resolve(opts),
            Self::Function(a) => a.resolve(opts),
        }
    }
}

pub struct LinkAction {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub copy: bool,
    pub optional: bool,
}

impl Resolvable for LinkAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            src,
            dest,
            copy,
            optional,
        } = self;

        // If optional flag enabled, and file does not exist, skip.
        if optional && !src.exists() {
            log_skipping(&format!(
                "{} does not exist",
                style(src.display()).underlined()
            ));
            return Ok(Resolution::Skipped);
        }

        // FIXME implement

        Ok(Resolution::Done)
    }
}

pub struct WriteAction {
    pub dest: PathBuf,
    pub contents: String,
}

impl Resolvable for WriteAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        // FIXME implement

        Ok(Resolution::Done)
    }
}

pub struct TreeAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

impl Resolvable for TreeAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            src,
            dest,
            globs,
            ignore,
            copy,
            optional,
        } = self;

        if !src.exists() {
            // If src does not exist, and optional flag enabled, skip.
            // If optional flag disabled, return error.
            if optional {
                log_skipping(&format!(
                    "{} does not exist",
                    style(src.display()).underlined()
                ));
                return Ok(Resolution::Skipped);
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{} was not found", src.display()),
                ))?;
            }
        } else if !src.is_dir() {
            // If src isn't a directory, and optional flag enabled, skip it.
            // If optional flag disabled, return error.
            if optional {
                log_skipping(&format!(
                    "{} is not a directory",
                    style(src.display()).underlined()
                ));
                return Ok(Resolution::Skipped);
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{} was not found", src.display()),
                ))?;
            }
        }

        // FIXME handle absolute path globs
        #[inline]
        fn glob_tree(
            src: impl AsRef<Path>,
            pats: &Vec<String>,
        ) -> Result<HashSet<PathBuf>, ResolveError> {
            let pats: Vec<_> = pats
                .iter()
                .map(|glob| format!("{}/{}", src.as_ref().display(), glob))
                .collect();
            let matches: Vec<glob::Paths> = pats
                .iter()
                .map(|pat| glob::glob(pat))
                .collect::<Result<_, _>>()?;

            let res = matches
                .into_iter()
                .flatten()
                .filter(|r| {
                    if let Ok(path) = r {
                        path.is_file()
                    } else {
                        false
                    }
                })
                .collect::<Result<_, _>>()?;

            Ok(res)
        }

        // Glob to get file paths.
        let mut paths = glob_tree(&src, &globs)?;
        // Glob to get ignored paths.
        let ignore_paths = glob_tree(&src, &ignore)?;

        // Remove all the ignored paths from the globbed paths.
        for path in ignore_paths {
            paths.remove(&path);
        }

        // FIXME better way to do this (e.g. chdir before globbing?)
        let rel_paths: Vec<_> = paths
            .iter()
            .map(|path| path.strip_prefix(&src))
            .collect::<Result<_, _>>()
            .unwrap();
        let dest_paths: Vec<_> = rel_paths.iter().map(|path| dest.join(path)).collect();

        // Map paths and dest paths into linking actions.
        let it = paths
            .into_iter()
            .zip(dest_paths.into_iter())
            .map(move |(fsrc, fdest)| {
                Action::Link(LinkAction {
                    src: fsrc,
                    dest: fdest,
                    copy,
                    optional: false,
                })
            });

        // FIXME handle resolutions
        it.map(|action| action.resolve(opts))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Resolution::Done)
    }
}

pub struct HandlebarsAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub vars: Tree,

    pub optional: bool,
    pub partials: HandlebarsPartials,
}

impl Resolvable for HandlebarsAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            src,
            dest,
            vars,
            optional,
            partials,
        } = self;

        // If optional flag enabled, and file does not exist, skip.
        if optional && !src.exists() {
            log_skipping(&format!(
                "{} does not exist",
                style(src.display()).underlined()
            ));
            return Ok(Resolution::Skipped);
        }

        // Render contents.
        let contents = templating::hbs::render(&src, &vars, &partials)?;

        let wa = WriteAction { dest, contents };
        wa.resolve(opts)
    }
}

pub struct LiquidAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub vars: Tree,

    pub optional: bool,
}

impl Resolvable for LiquidAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            src,
            dest,
            vars,
            optional,
        } = self;

        // If optional flag enabled, and file does not exist, skip.
        if optional && !src.exists() {
            log_skipping(&format!(
                "{} does not exist",
                style(src.display()).underlined()
            ));
            return Ok(Resolution::Skipped);
        }

        let contents = templating::liquid::render(&src, &vars)?;

        let wa = WriteAction { dest, contents };
        wa.resolve(opts)
    }
}

pub struct YamlAction {
    pub dest: PathBuf,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolvable for YamlAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = serde_yaml::to_string(&values)?;
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts)
    }
}

pub struct TomlAction {
    pub dest: PathBuf,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolvable for TomlAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = toml::to_string_pretty(&values)?;
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts)
    }
}

pub struct JsonAction {
    pub dest: PathBuf,
    pub values: Tree,
}

impl Resolvable for JsonAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        let Self { dest, values } = self;

        let contents = serde_json::to_string(&values)?;

        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        wa.resolve(opts)
    }
}

pub struct CommandAction {
    pub command: String,

    pub quiet: bool,
    pub start: PathBuf,
    pub shell: String,
}

impl Resolvable for CommandAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        // FIXME implement
        Ok(Resolution::Done)
    }
}

pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,
    pub quiet: bool,
}

impl<'a> Resolvable for FunctionAction<'a> {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, ResolveError> {
        // FIXME implement
        Ok(Resolution::Done)
    }
}

#[inline]
fn log_processing(idxl: &Indexed, step: &str) {
    idxl.debug(&format!("Processing: {}", step));
}

#[inline]
fn log_skipping(reason: &str) {
    Sublevel::default().debug(&format!("{} {}", style("Skipping...").bold(), reason));
}
