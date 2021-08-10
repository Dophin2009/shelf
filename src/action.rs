use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use glob::{GlobError, PatternError};
use mlua::Function;

use crate::error::EmptyError;
use crate::format::{self, errored, style, sublevel};
use crate::spec::{EnvMap, HandlebarsPartials, NonZeroExitBehavior, Patterns};
use crate::templating;
use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolve {
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError>;
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

impl<'a> Resolve for Action<'a> {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
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

impl Resolve for LinkAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            src,
            dest,
            copy,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        if !src.exists() {
            if optional {
                log_skipping(format!(
                    "{} does not exist",
                    format::filepath(src.display())
                ));
                return Ok(Resolution::Skipped);
            } else {
                log_missing(&src);
                return Err(EmptyError);
            }
        }

        // FIXME implement

        Ok(Resolution::Done)
    }
}

pub struct WriteAction {
    pub dest: PathBuf,
    pub contents: String,
}

impl Resolve for WriteAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
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

impl Resolve for TreeAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            src,
            dest,
            globs,
            ignore,
            copy,
            optional,
        } = self;

        // If src does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        // If src exists but isn't a directory, and optional flag enabled, skip it.
        // If optional flag disabled, return error.
        if !src.exists() || !src.is_dir() {
            if optional {
                log_skipping(format!(
                    "{} does not exist",
                    format::filepath(src.display())
                ));
                return Ok(Resolution::Skipped);
            } else {
                log_missing(&src);
                return Err(EmptyError);
            }
        }

        // FIXME handle absolute path globs
        #[inline]
        fn glob_tree(
            src: impl AsRef<Path>,
            pats: &Vec<String>,
        ) -> Result<HashSet<PathBuf>, EmptyError> {
            let pats: Vec<_> = pats
                .iter()
                .map(|glob| format!("{}/{}", src.as_ref().display(), glob))
                .collect();
            let matches: Vec<glob::Paths> = pats
                .iter()
                .map(|pat| glob::glob(pat))
                .map(|r| {
                    r.map_err(|err| {
                        errored::error(format!(
                            "{} {}",
                            style("Couldn't glob a pattern:").red(),
                            err
                        ));
                        EmptyError
                    })
                })
                .collect::<Result<_, _>>()?;

            let res = matches
                .into_iter()
                .flatten()
                .filter_map(|r| match r {
                    Ok(path) if path.is_file() => Some(Ok(path)),
                    Ok(path) => None,
                    Err(err) => {
                        errored::error(format!(
                            "{} {}",
                            style("Couldn't read path while globbing:").red(),
                            err
                        ));
                        Some(Err(EmptyError))
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

impl Resolve for HandlebarsAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            src,
            dest,
            vars,
            optional,
            partials,
        } = self;

        // If file does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        if !src.exists() {
            if optional {
                log_skipping(&format!(
                    "{} does not exist",
                    format::filepath(src.display())
                ));
                return Ok(Resolution::Skipped);
            } else {
                log_missing(&src);
                return Err(EmptyError);
            }
        }

        // Render contents.
        let contents = fail!(templating::hbs::render(&src, &vars, &partials), err => {
            errored::error(format!("{} {}", style("Couldn't render Handlebars template:").red(), err));
        });
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

impl Resolve for LiquidAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            src,
            dest,
            vars,
            optional,
        } = self;

        // If file does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        if !src.exists() {
            if optional {
                log_skipping(format!(
                    "{} does not exist",
                    format::filepath(src.display())
                ));
                return Ok(Resolution::Skipped);
            } else {
                log_missing(&src);
                return Err(EmptyError);
            }
        }

        // Render resulting file contents.
        let contents = fail!(templating::liquid::render(&src, &vars), err => {
            errored::error(format!("{} {}", style("Couldn't render Liquid template:").red(), err));
        });
        let wa = WriteAction { dest, contents };
        wa.resolve(opts)
    }
}

pub struct YamlAction {
    pub dest: PathBuf,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolve for YamlAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(serde_yaml::to_string(&values), err => {
            errored::error(format!("{} {}", style("Couldn't convert value map into yaml:").red(), err))
        });
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

impl Resolve for TomlAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(toml::to_string_pretty(&values), err => {
            errored::error(format!("{} {}", style("Couldn't convert value map into toml:").red(), err))
        });
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

impl Resolve for JsonAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self { dest, values } = self;

        let contents = fail!(serde_json::to_string(&values), err => {
            errored::error(format!("{} {}", style("Couldn't convert value map into json:").red(), err))
        });

        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        wa.resolve(opts)
    }
}

pub struct CommandAction {
    pub command: String,

    pub start: PathBuf,
    pub shell: String,

    pub stdout: bool,
    pub stderr: bool,

    pub clean_env: bool,
    pub env: EnvMap,

    pub nonzero_exit: NonZeroExitBehavior,
}

impl Resolve for CommandAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        let Self {
            command,
            start,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit,
        } = self;

        sublevel::debug("Building command...");

        let mut cmd = Command::new(shell);
        cmd.args(&["-c", &command]).current_dir(&start);

        if !stdout {
            sublevel::debug("Capturing stdout...");
            cmd.stdout(Stdio::null());
        }
        if !stderr {
            sublevel::debug("Capturing stderr...");
            cmd.stderr(Stdio::null());
        }

        if clean_env {
            sublevel::debug("Clearing environment variables...");
            cmd.env_clear();
        }

        if !env.is_empty() {
            sublevel::debug("Populating environment variables...");
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        sublevel::debug("Spawning...");
        let mut child = fail!(cmd.spawn(), err => {
            errored::error(format!("{} {}", style("Couldn't spawn command:").red(), err));
        });

        let res = fail!(child.wait(), err => {
            errored::error(format!("{} {}", style("Couldn't finish command:").red(), err));
        });

        if let Some(code) = res.code() {
            sublevel::debug(format!("Done... exit {}", style(code).green()));
        }

        // Check for non zero exit status.
        if !res.success() {
            match nonzero_exit {
                NonZeroExitBehavior::Error => {
                    errored::error(format!(
                        "{} '{}' {}",
                        style("Hook").red(),
                        style(command).dim(),
                        style("exited with a non-zero status").red()
                    ));
                    return Err(EmptyError);
                }
                NonZeroExitBehavior::Warn => sublevel::warn(format!(
                    "{} '{}' {}",
                    style("Hook").yellow(),
                    style(command).dim(),
                    style("exited with a non-zero status").yellow()
                )),
                NonZeroExitBehavior::Ignore => {}
            }
        }

        Ok(Resolution::Done)
    }
}

pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,
    pub quiet: bool,
}

impl<'a> Resolve for FunctionAction<'a> {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> Result<Resolution, EmptyError> {
        // FIXME implement
        Ok(Resolution::Done)
    }
}

#[inline]
fn log_missing(path: impl AsRef<Path>) {
    errored::error(format!(
        "{} {} does not exist",
        style("Failed!").red(),
        format::filepath(path.as_ref().display())
    ));
}

#[inline]
fn log_skipping<M>(reason: M)
where
    M: fmt::Display,
{
    sublevel::debug(format!("{} {}", style("Skipping...").yellow(), reason));
}
