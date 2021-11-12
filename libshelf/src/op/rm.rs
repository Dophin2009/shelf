use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::{Finish, Rollback};

sa::assert_impl_all!(RmOp: Finish<Output = RmFinish, Error = RmOpError>);
sa::assert_impl_all!(RmFinish: Rollback<Output = RmUndoOp>);
sa::assert_impl_all!(RmUndoOp: Finish<Output = RmUndoFinish, Error = RmOpError>);
sa::assert_impl_all!(RmUndoFinish: Rollback<Output = RmOp>);

/// Error encountered when finishing [`RmOp`] or [`RmUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum RmOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
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

// TODO: Undoing directory and contents deletion?
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmOp {
    /// Path at which the file or directory will be removed.
    pub path: PathBuf,
    /// If true, finishing will try to delete a directory. This must be set to `true` if `path` is
    /// a directory.
    pub dir: bool,
}

/// The output of [`RmOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,
}

impl Finish for RmOp {
    type Output = RmFinish;
    type Error = RmOpError;

    // TODO: Move file to file safe
    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path, dir } = self;
        if *dir {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        };

        Ok(Self::Output {
            path: path.clone(),
            dir: *dir,
        })
    }
}

impl Rollback for RmFinish {
    type Output = RmUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path, dir } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
        }
    }
}

/// The undo of [`RmOp`] (see its documentation), created by rolling back [`RmFinish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmUndoOp {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,
}

/// The output of [`RmUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmUndoFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,
}

impl Finish for RmUndoOp {
    type Output = RmUndoFinish;
    type Error = RmOpError;

    // FIXME: Pass in a saved directory and implement.
    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path: _, dir: _ } = self;

        unimplemented!();
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
