pub use crate::spec::Patterns;

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use glob::{GlobError, PatternError};

use crate::fsutil;

use super::error::FileMissingError;
use super::{Error, LinkAction, Res, Resolve, ResolveOpts, SkipReason};

#[derive(Debug, Clone)]
pub struct TreeAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum TreeActionError {
    #[error("glob error")]
    Glob(#[from] GlobError),
    #[error("pattern error")]
    Pattern(#[from] PatternError),
    #[error("file missing")]
    FileMissing(#[from] FileMissingError),
    #[error("link action error")]
    Link(#[from] Error),
}

impl<'lua> Resolve<'lua> for TreeAction {
    type Error = TreeActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let Self {
            src,
            dest,
            globs,
            ignore,
            copy,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (optional, fsutil::exists(src)) {
            (true, false) => {
                return Ok(Res::Skip(SkipReason::OptionalMissing { path: src.clone() }));
            }
            (false, false) => {
                return Err(TreeActionError::FileMissing(FileMissingError {
                    path: src.clone(),
                }));
            }
            _ => {}
        }

        // Glob to get file paths.
        let mut paths = Self::glob_tree(&src, globs)?;
        // Glob to get ignored paths.
        let ignore_paths = Self::glob_tree(&src, ignore)?;

        // Remove all the ignored paths from the globbed paths.
        for path in ignore_paths {
            paths.remove(&path);
        }

        // Join these back into full paths for src and dest.
        let src_paths = paths.iter().map(|path| src.join(path));
        let dest_paths = paths.iter().map(|path| dest.join(path));

        // Map paths and dest paths into linking actions.
        let it = src_paths
            .zip(dest_paths)
            .map(move |(fsrc, fdest)| LinkAction {
                src: fsrc,
                dest: fdest,
                copy: *copy,
                optional: false,
            });

        let resolutions: Vec<_> = it
            .map(|action| action.resolve(opts))
            .collect::<Result<_, _>>()?;
        Ok(Res::Multiple(resolutions))
    }
}

impl TreeAction {
    // FIXME: handle absolute path globs
    #[inline]
    fn glob_tree<P>(src: P, pats: &[String]) -> Result<HashSet<PathBuf>, TreeActionError>
    where
        P: AsRef<Path>,
    {
        let cwd = env::current_dir().unwrap();
        env::set_current_dir(&src).unwrap();

        let matches: Vec<glob::Paths> = pats
            .iter()
            .map(|pat| glob::glob(pat))
            .collect::<Result<_, _>>()?;

        let res = matches
            .into_iter()
            .flatten()
            .filter_map(|r| match r {
                // FIXME: ??
                Ok(path) if path.is_file() => Some(Ok(path)),
                Ok(_) => None,
                Err(err) => Some(Err(err)),
            })
            .collect::<Result<_, _>>()?;

        env::set_current_dir(&cwd).unwrap();

        Ok(res)
    }
}
