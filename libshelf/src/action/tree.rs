use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{env, fs};

use glob::{GlobError, PatternError};

use crate::fsutil;

use super::error::FileMissingError;
use super::link::Res as LinkActionRes;
use super::{LinkAction, Resolve};

pub type Patterns = Vec<Pattern>;
pub type Pattern = String;

#[derive(Debug, Clone)]
pub struct TreeAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub enum Res {
    // TODO: Better API than this?
    Normal(Vec<LinkActionRes>),
    /// The action is skipped.
    Skip(Skip),
}

/// Reason for skipping [`TreeAction`].
#[derive(Debug, Clone)]
pub enum Skip {
    /// `src` and `dest` are the same path.
    SameSrcDest,
    /// Optional `src` does not exist.
    OptMissing,
    /// Destination link already exists.
    DestExists,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("src missing")]
    SrcMissing,
    #[error("glob error")]
    Glob(#[from] GlobError),
    #[error("pattern error")]
    Pattern(#[from] PatternError),
}

impl Resolve for TreeAction {
    type Output = Result<Res, Error>;

    #[inline]
    fn resolve(&self) -> Self::Output {
        let Self {
            src,
            dest,
            globs,
            ignore,
            copy,
            optional,
        } = self;

        match (optional, fsutil::exists(src)) {
            // `src` is optional and does not exist; skip.
            (true, false) => {
                return Ok(Res::Skip(Skip::OptMissing));
            }
            // `src` is not optional but does not exist; skip.
            (false, false) => {
                return Err(Error::SrcMissing);
            }
            // Otherwise, `src` exists; continue.
            _ => {}
        };

        // Glob to get file paths.
        let mut paths = glob_tree(&src, globs)?;
        // Glob to get ignored paths.
        let ignore_paths = glob_tree(&src, ignore)?;

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

        // SAFETY: Should be fine since all these files should exist?
        let resvec: Vec<_> = it.map(|action| action.resolve(opts).unwrap());
        Ok(Res::Normal(resvec))
    }
}

#[inline]
fn glob_tree<P>(src: P, pats: &[String]) -> Result<HashSet<PathBuf>, Error>
where
    P: AsRef<Path>,
{
    // TODO: Better way to do this than chdir?
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
            Ok(path) => Some(path).filter(keep_globbed),
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<_, _>>()?;

    env::set_current_dir(&cwd).unwrap();

    Ok(res)
}

#[inline]
fn keep_globbed<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    match fs::symlink_metadata(path) {
        Ok(meta) => meta.is_file(),
        // In case of error, just don't keep.
        Err(_) => false,
    }
}
