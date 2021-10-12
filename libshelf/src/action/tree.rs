use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use glob::GlobError;

use crate::fsutil;

use super::{LinkAction, Patterns, Resolution, Resolve, ResolveOpts};

#[derive(Debug, Clone)]
pub struct TreeAction {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TreeActionError {
    #[error("glob error")]
    Glob(#[from] GlobError),
}

impl Resolve for TreeAction {
    type Error = TreeActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution, Self::Error> {
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
                return Ok(Resolution::Skip(SkipReason::OptionalMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(ResolutionError::FileMissing { path: src.clone() });
            }
            _ => {}
        }

        // FIXME: handle absolute path globs
        #[inline]
        fn glob_tree<P>(src: P, pats: &[String]) -> Result<HashSet<PathBuf>, GlobError>
        where
            P: AsRef<Path>,
        {
            let cwd = env::current_dir().unwrap();
            env::set_current_dir(src.abs()).unwrap();

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

        // Glob to get file paths.
        let mut paths = glob_tree(&src, &globs)?;
        // Glob to get ignored paths.
        let ignore_paths = glob_tree(&src, &ignore)?;

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
                copy,
                optional: false,
            });

        let resolutions: Vec<_> = it
            .map(|action| action.resolve(opts))
            .collect::<Result<_, _>>()?;
        Ok(Resolution::Multiple(resolutions))
    }
}
