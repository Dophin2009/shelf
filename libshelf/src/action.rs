use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use glob::{GlobError, PatternError};
use mlua::Function;

use crate::cache::{Cache, FsCache};
use crate::error::EmptyError;
use crate::pathutil::PathWrapper;
use crate::spec::{EnvMap, HandlebarsPartials, NonZeroExitBehavior, Patterns};
use crate::templating;
use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolve {
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError>;
}

pub enum Resolution {
    Done,
    Skipped,
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
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        match self {
            Self::Link(a) => a.resolve(opts, cache),
            Self::Write(a) => a.resolve(opts, cache),
            Self::Tree(a) => a.resolve(opts, cache),
            Self::Handlebars(a) => a.resolve(opts, cache),
            Self::Liquid(a) => a.resolve(opts, cache),
            Self::Yaml(a) => a.resolve(opts, cache),
            Self::Toml(a) => a.resolve(opts, cache),
            Self::Json(a) => a.resolve(opts, cache),
            Self::Command(a) => a.resolve(opts, cache),
            Self::Function(a) => a.resolve(opts, cache),
        }
    }
}

macro_rules! log_skip {
    () => {
        sl_debug!("")
    };
    ($format_str:literal $(, $arg:expr)* $(,)?) => {
        log_skip!([$format_str] $(, $arg )*)
    };
    ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
        sl_debug!(["{$yellow}Skipping...{/$} ", $($format_str),+] $(, $arg)*)
    };
}

pub struct LinkAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,

    pub copy: bool,
    pub optional: bool,
}

impl Resolve for LinkAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self {
            src,
            dest,
            copy,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (src.exists(), optional) {
            (false, true) => {
                log_skip!("{[green]} does not exist", src.reld());
                return Ok(Resolution::Skipped);
            }
            (false, false) => {
                log_miss(&src);
                return Err(EmptyError);
            }
            _ => {}
        };

        if copy {
            Self::copy_file(&src, &dest, opts, cache)?;
        } else {
            Self::symlink_file(&src, &dest, opts, cache)?;
        }

        // FIXME cache this action

        Ok(Resolution::Done)
    }
}

impl LinkAction {
    // FIXME implement missing pieces
    #[inline]
    fn copy_file(
        src: &PathWrapper,
        dest: &PathWrapper,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<(), EmptyError> {
        let dest_cache = cache.get(dest.abs().to_path_buf());
        // Check the cache for the destination path.
        match (dest.exists(), dest_cache) {
            (true, Some(dest_cache)) => match dest_cache.typ {},
            (false, Some(dest_cache)) => {
                // Destination file doesn't exist but found in cache; warn about changes that might've
                // occurred since last run.
                sl_warn!("{$yellow}Destination was found in cache, but no file exists there.{/$}");
                sl_warn!("{$yellow}Manual changes may have removed this file since last time.{/$}");
                sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
            }
            (true, None) => {
                // Destination file exists but not found in cache, warn about overwriting.
                sl_warn!(
                    "{$yellow}Destination wasn't found in the cache, but a file exists there.{/$}"
                );
                sl_warn!("{$yellow}It will be replaced.{/$}");
                sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
            }
            (false, None) => {
                // Destination doesn't have a file and not found in cache; continue to copying.
            }
        };

        // If the content hash doesn't match the new content, emit a warning/error.

        // If the content has does match the new content, do nothing.

        sl_debug!(
            "Copying file: {[green]} to {[green]}",
            src.reld(),
            dest.reld()
        );

        // Make the parent directories.
        mkdir_parents(dest)?;

        // Actually copy.
        let res = fs::copy(&src.abs(), &dest.abs());
        fail!(res, err => {
            sl_error!("{$red}Couldn't copy file:{/$} {}", err);
            sl_i_error!("{$red}Source:{/$} {[green]}", src.absd());
            sl_i_error!("{$red}Destination:{/$} {[green]}", dest.absd());
        });

        Ok(())
    }

    #[inline]
    fn symlink_file(
        src: &PathWrapper,
        dest: &PathWrapper,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<(), EmptyError> {
        // Inspect the file metadata of the destination path.
        // If the destination doesn't exist, just skip to symlinking.
        if dest.exists() {
            sl_debug!("Found an existing file at the destination; checking...");

            let meta = fail!(fs::symlink_metadata(dest.abs()), err => {
                sl_error!("{$red}Couldn't read file metadata:{/$} {}", err);
                sl_i_error!("{$red}{/$}");
            });

            let ft = meta.file_type();
            // If it's a symlink, read the target location and check against src path.
            // Path must be the same, not just location the same.
            if ft.is_symlink() {
                let starget = fail!(fs::read_link(dest.abs()), err => {
                    sl_error!("{$red}Couldn't follow symlink:{/$} {}", err);
                    sl_i_error!("{$red}Location:{/$} {[green]}", dest.absd());
                });

                if starget == *src.abs() {
                    // src and current destination symlink are the same; emit a notice.
                    sl_debug!("Symlink was already established; doing nothing...");
                    return Ok(());
                } else {
                    // FIXME warn/error depend on opts
                    // src and current destination symlink are different.
                    // Emit a warning/error and delete the symlink.
                    sl_warn!("{$yellow}An existing symlink (pointing to a different location) was found at the destination{/$}");
                    sl_i_warn!("{$yellow}Destination:{/$} {[green]}", dest.absd());
                    sl_i_warn!("{$yellow}It will be replaced.{/$}");
                    fail!(fs::remove_file(dest.abs()), err => {
                        sl_error!("{$red}Couldn't delete the symlink:{/$} {}", err);
                        sl_i_error!("{$red}Destination:{/$} {[green]}", dest.absd());
                    });
                }
            } else {
                // Not a symlink, so emit a warning/error.
                sl_warn!("{$yellow}An existing file or directory was found at the destination{/$}");
                sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
                sl_i_warn!("{$yellow}It will be replaced.{/$}");

                // FIXME should be based on options
                if dest.is_file() {
                    fail!(fs::remove_file(dest.abs()), err => {
                        sl_error!("{$red}Couldn't delete the file:{/$} {}", err);
                        sl_i_error!("{$yellow}Location:{/$} {[green]}", dest.absd());
                    });
                } else if dest.is_dir() {
                    fail!(fs::remove_dir_all(dest.abs()), err => {
                        sl_error!("{$red}Couldn't delete the directory:{/$} {}", err);
                        sl_i_error!("{$yellow}Location:{/$} {[green]}", dest.absd());
                    });
                }
            }
        }

        sl_debug!(
            "Linking file: {[green]} to {[green]}",
            src.reld(),
            dest.reld()
        );

        let res = inner_symlink_file(src, dest);
        fail!(res, err => {
            sl_error!("{$red}Couldn't symlink:{/$} {}", err);
            sl_i_error!("Source: {[green]}",  src.absd());
            sl_i_error!("Destination: {[green]}", dest.absd());
        });

        Ok(())
    }
}

#[cfg(unix)]
#[inline]
fn inner_symlink_file(src: &PathWrapper, dest: &PathWrapper) -> io::Result<()> {
    use std::os::unix;
    unix::fs::symlink(src.abs(), dest.abs())
}

// FIXME implement
#[cfg(windows)]
#[inline]
fn inner_symlink_file(src: &PathWrapper, dest: &PathWrapper) -> io::Result<()> {
    todo!()
}

pub struct WriteAction {
    pub dest: PathWrapper,
    pub contents: String,
}

impl Resolve for WriteAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self { dest, contents } = self;

        // If the destination doesn't exist yet, create the directories and write the file.
        if !dest.exists() {
            mkdir_parents(&dest)?;

            sl_debug!("Writing file: {[green]}", dest.reld());
            fail!(fs::write(&dest.abs(), &contents), err => {
                sl_error!("{$red}Couldn't write{/$} {[green]} {$red}:{/$} {}", dest.absd(), err);
            });

            sl_info!("Done... {$green}ok!{/$}");

            // FIXME cache this action
            Ok(Resolution::Done)
        } else {
            todo!();

            // Retrieve information for this location from the cache.
            // If not found, we should error.

            // If existing is a symlink, replace it.

            // If existing is not a symlink, replace it.

            // Cache the action.

            Ok(Resolution::Done)
        }
    }
}

#[inline]
fn mkdir_parents(path: &PathWrapper) -> Result<(), EmptyError> {
    // FIXME how to handle this case? (i.e. where path is /)
    let parent = failopt!(path.parent(), { todo!() });

    if !parent.exists() {
        sl_debug!("Creating directories: {[green]}", parent.reld());

        fail!(fs::create_dir_all(parent.abs()), err => {
            sl_error!("{$red}Couldn't create parent directories at{/$} {[green]} {$red}:{/$} {}", parent.absd(), err);
        });
    }

    Ok(())
}

pub struct TreeAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

impl Resolve for TreeAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
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
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skipped);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // FIXME handle absolute path globs
        #[inline]
        fn glob_tree(
            src: &PathWrapper,
            pats: &Vec<String>,
        ) -> Result<HashSet<PathBuf>, EmptyError> {
            let cwd = env::current_dir().unwrap();
            env::set_current_dir(src.abs()).unwrap();

            let matches: Vec<glob::Paths> = pats
                .iter()
                .map(|pat| glob::glob(pat))
                .map(|r| {
                    r.map_err(|err| {
                        // FIXME path in error
                        sl_error!("{$red}Couldn't glob a pattern:{/$} {}", err);
                        EmptyError
                    })
                })
                .collect::<Result<_, _>>()?;

            let res = matches
                .into_iter()
                .flatten()
                .filter_map(|r| match r {
                    Ok(path) if path.is_file() => Some(Ok(path)),
                    Ok(_) => None,
                    Err(err) => {
                        // FIXME path in error
                        sl_error!("{$red}Couldn't read path while globbing:{/$} {}", err);
                        Some(Err(EmptyError))
                    }
                })
                .collect::<Result<_, _>>()?;

            env::set_current_dir(&cwd).unwrap();

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

        let src_paths = paths.iter().map(|path| src.join(path));
        let dest_paths = paths.iter().map(|path| dest.join(path));

        // Map paths and dest paths into linking actions.
        let it = src_paths.zip(dest_paths).map(move |(fsrc, fdest)| {
            Action::Link(LinkAction {
                src: fsrc,
                dest: fdest,
                copy,
                optional: false,
            })
        });

        // FIXME handle resolutions
        it.map(|action| action.resolve(opts, cache))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Resolution::Done)
    }
}

pub struct HandlebarsAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub vars: Tree,

    pub optional: bool,
    pub partials: HandlebarsPartials,
}

impl Resolve for HandlebarsAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
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
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skipped);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // Render contents.
        let contents = fail!(templating::hbs::render(src.abs(), &vars, &partials), err => {
            sl_error!("{$red}Couldn't render Handlebars template:{/$} {}", err);
        });
        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct LiquidAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub vars: Tree,

    pub optional: bool,
}

impl Resolve for LiquidAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
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
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skipped);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // Render resulting file contents.
        let contents = fail!(templating::liquid::render(src.abs(), &vars), err => {
            sl_error!("{$red}Couldn't render Liquid template:{/$} {}", err);
        });
        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct YamlAction {
    pub dest: PathWrapper,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolve for YamlAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(serde_yaml::to_string(&values), err => {
            sl_error!("{$red}Couldn't convert value map into yaml:{/$} {}", err);
        });
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct TomlAction {
    pub dest: PathWrapper,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolve for TomlAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(toml::to_string_pretty(&values), err => {
            sl_error!("{$red}Couldn't convert value map into toml:{/$} {}", err);
        });
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct JsonAction {
    pub dest: PathWrapper,
    pub values: Tree,
}

impl Resolve for JsonAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self { dest, values } = self;

        let contents = fail!(serde_json::to_string(&values), err => {
            sl_error!("{$red}Couldn't convert value map into json:{/$} {}", err);
        });

        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        wa.resolve(opts, cache)
    }
}

pub struct CommandAction {
    pub command: String,

    pub start: PathWrapper,
    pub shell: String,

    pub stdout: bool,
    pub stderr: bool,

    pub clean_env: bool,
    pub env: EnvMap,

    pub nonzero_exit: NonZeroExitBehavior,
}

impl Resolve for CommandAction {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        _cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
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

        sl_debug!("Building command...");

        let mut cmd = Command::new(shell);
        cmd.args(&["-c", &command]).current_dir(&start.abs());

        if !stdout {
            sl_debug!("Capturing stdout...");
            cmd.stdout(Stdio::null());
        }
        if !stderr {
            sl_debug!("Capturing stderr...");
            cmd.stderr(Stdio::null());
        }

        if clean_env {
            sl_debug!("Clearing environment variables...");
            cmd.env_clear();
        }

        if !env.is_empty() {
            sl_debug!("Populating environment variables...");
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        sl_debug!("Spawning...");
        let mut child = fail!(cmd.spawn(), err => {
            sl_error!("{$red}Couldn't spawn command:{/$} {}", err);
        });

        let res = fail!(child.wait(), err => {
            sl_error!("{$red}Couldn't finish command:{/$} {}", err);
        });

        if let Some(code) = res.code() {
            sl_debug!("Done... exit {[green]}", code);
        }

        // Check for non zero exit status.
        if !res.success() {
            match nonzero_exit {
                NonZeroExitBehavior::Error => {
                    sl_error!(
                        "{$red}Hook{/$} '{[dimmed]}' {$red}exited with a non-zero status{/$}",
                        command
                    );
                    return Err(EmptyError);
                }
                NonZeroExitBehavior::Warn => sl_warn!(
                    "{$yellow}Hook{/$} '{[dimmed]}' {$yellow}exited with a non-zero status{/$}",
                    command,
                ),
                NonZeroExitBehavior::Ignore => {}
            };
        }

        Ok(Resolution::Done)
    }
}

pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,

    pub start: PathWrapper,
    pub error_exit: NonZeroExitBehavior,
}

impl<'a> Resolve for FunctionAction<'a> {
    #[inline]
    fn resolve(
        self,
        opts: &ResolveOpts,
        _cache: &mut Box<dyn Cache>,
    ) -> Result<Resolution, EmptyError> {
        let Self {
            function,
            start,
            error_exit,
        } = self;

        let cwd = env::current_dir().unwrap();
        env::set_current_dir(start.abs()).unwrap();

        sl_debug!("Calling function...");
        let ret: mlua::Value = fail!(function.call(()), err => {
            sl_error!("{$red}Couldn't finish executing function hook:{/$} {}", err);
        });

        match ret {
            mlua::Value::Nil => {}
            v => match error_exit {
                NonZeroExitBehavior::Error => {
                    sl_error!("{$red}Function returned with an error:{/$} {:?}", v);
                    return Err(EmptyError);
                }
                NonZeroExitBehavior::Warn => {
                    sl_warn!("Done... {$yellow}non-nil exit:{/$} {:?}", v)
                }
                NonZeroExitBehavior::Ignore => {
                    sl_debug!("Done... exit {$blue}nil{/$}");
                }
            },
        }

        // FIXME restore cwd regardless of error or not
        env::set_current_dir(&cwd).unwrap();
        Ok(Resolution::Done)
    }
}

#[inline]
fn log_miss(path: &PathWrapper) {
    sl_error!("{$red}Failed!{/$} Missing file: {[green]}", path.absd());
}
