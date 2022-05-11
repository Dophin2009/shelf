use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::RenameError;
use super::{Finish, Rollback};

sa::assert_impl_all!(RmOp: Finish<Output = RmFinish, Error = RmOpError>);
sa::assert_impl_all!(RmFinish: Rollback<Output = RmUndoOp>);
sa::assert_impl_all!(RmUndoOp: Finish<Output = RmUndoFinish, Error = RmUndoOpError>);
sa::assert_impl_all!(RmUndoFinish: Rollback<Output = RmOp>);

/// Error encountered when finishing [`RmOp`].
#[derive(Debug, thiserror::Error)]
pub enum RmOpError {
    #[error("rename error")]
    Rename(#[from] RenameError),
}

/// Operation to remove a file or directory at `path`.
///
/// # Errors
///
/// The operation will error if there is no existing file at `path` or there are insufficient
/// permissions. If there is a directory at `path`, but `dir` is false, an error will occur.
///
/// # Undo
///
/// Undoing will restore the file or directory (and its contents). This set of operations functions
/// in the following cycle:
///
/// [`RmOp`] --> [`RmFinish`] --> [`RmUndoOp`] --> [`RmUndoFinish`] --> [`RmOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmOp {
    /// Path at which the file or directory will be removed.
    pub path: PathBuf,
    /// If true, finishing will try to delete a directory. This must be set to `true` if `path` is
    /// a directory.
    pub dir: bool,
}

/// The output of [`RmOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,

    /// Path at which the file was backed-up.
    pub safepath: PathBuf,
}

impl Finish for RmOp {
    type Output = RmFinish;
    type Error = RmOpError;

    // TODO: What about directories? Calling `fs::create_dir` would make this two operations aaaahh
    // this whole design sucks
    #[inline]
    fn finish(&self, ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path, dir } = self;

        let safepath = ctx.filesafe.resolve(path);
        fs::rename(path, &safepath).map_err(|inner| RenameError {
            src: path.clone(),
            dest: safepath.clone(),
            inner,
        })?;

        Ok(Self::Output {
            path: path.clone(),
            dir: *dir,
            safepath,
        })
    }
}

impl Rollback for RmFinish {
    type Output = RmUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self {
            path,
            dir,
            safepath,
        } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
            safepath: safepath.clone(),
        }
    }
}

/// Error encountered when finishing [`RmUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum RmUndoOpError {
    #[error("rename error")]
    Rename(#[from] RenameError),
}

/// The undo of [`RmOp`] (see its documentation), created by rolling back [`RmFinish`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmUndoOp {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,

    /// See [`RmFinish`].
    pub safepath: PathBuf,
}

/// The output of [`RmUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmUndoFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,
}

impl Finish for RmUndoOp {
    type Output = RmUndoFinish;
    type Error = RmUndoOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self {
            path,
            dir,
            safepath,
        } = self;

        // TODO: Fall back to copying and removing if renaming fails (e.g. cross-device link).
        fs::rename(safepath, path).map_err(|inner| RenameError {
            src: safepath.clone(),
            dest: path.clone(),
            inner,
        })?;

        Ok(Self::Output {
            path: path.clone(),
            dir: *dir,
        })
    }
}

impl Rollback for RmUndoFinish {
    type Output = RmOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path, dir } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fse;

    use super::super::test;
    use super::{Finish, RmOp, RmOpError, Rollback};

    /// Test regular file.
    #[test]
    fn test_regular_file() -> test::Result<()> {
        test::with_tempdir(|dir, ctx| {
            let (_, path) = test::new_file(dir, "a")?;

            let op = RmOp {
                path: path.clone(),
                dir: false,
            };

            let opf = op.finish(ctx)?;
            assert!(!fse::symlink_exists(&path));
            assert!(fse::symlink_exists(&opf.safepath));

            let undo = opf.rollback();
            let undof = undo.finish(ctx)?;
            assert!(fse::symlink_exists(&path));
            assert!(!fse::symlink_exists(&opf.safepath));

            let op2 = undof.rollback();
            assert_eq!(op, op2);

            Ok(())
        })
    }

    /// Test for nonexistent file.
    #[test]
    fn test_nonexistent_file() -> test::Result<()> {
        test::with_tempdir(|dir, ctx| {
            let path = dir.join("a");
            let op = RmOp {
                path: path.clone(),
                dir: false,
            };

            match op.finish(ctx) {
                Ok(_) => panic!("op succeeded"),
                Err(err) => match err {
                    RmOpError::Rename(err) => {
                        assert_eq!(err.src, path);
                    }
                },
            }

            Ok(())
        })
    }
}
