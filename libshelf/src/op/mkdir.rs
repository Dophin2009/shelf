use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::MkdirError;
use super::error::RemoveError;
use super::{Finish, Rollback};

sa::assert_impl_all!(MkdirOp: Finish<Output = MkdirFinish, Error = MkdirOpError>);
sa::assert_impl_all!(MkdirFinish: Rollback<Output = MkdirUndoOp>);
sa::assert_impl_all!(MkdirUndoOp: Finish<Output = MkdirUndoFinish, Error = MkdirOpError>);
sa::assert_impl_all!(MkdirUndoFinish: Rollback<Output = MkdirOp>);

/// Error encountered when finishing [`MkdirOp`] or [`MkdirUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum MkdirOpError {
    #[error("mkdir error")]
    Mkdir(#[from] MkdirError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
}

/// Operation to create a directory at `path`.
///
/// # Errors
///
/// The operation will error if `path` points to an existing file or if `path` is not writable.
///
/// # Undo
///
/// Undoing will delete the directory, erroring if it is not empty. This set of operations
/// functions in the following cycle:
///
/// [`MkdirOp`] --> [`MkdirFinish`] --> [`MkdirUndoOp`] --> [`MkdirUndoFinish`] --> [`MkdirOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirOp {
    /// Path at which the directory will be created.
    pub path: PathBuf,
}

/// The output of [`MkdirOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirFinish {
    /// See [`MkdirOp`].
    pub path: PathBuf,
}

impl Finish for MkdirOp {
    type Output = MkdirFinish;
    type Error = MkdirOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        fs::create_dir(path).map_err(|inner| MkdirError {
            path: path.clone(),
            inner,
        })?;
        Ok(Self::Output { path: path.clone() })
    }
}

impl Rollback for MkdirFinish {
    type Output = MkdirUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}

/// The undo of [`MkdirOp`] (see its documentation), created by rolling back [`MkdirFinish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirUndoOp {
    /// See [`MkdirOp`].
    pub path: PathBuf,
}

/// The output of [`MkdirUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirUndoFinish {
    /// See [`MkdirOp`].
    pub path: PathBuf,
}

impl Finish for MkdirUndoOp {
    type Output = MkdirUndoFinish;
    type Error = MkdirOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        fs::remove_dir(path).map_err(|inner| RemoveError {
            path: path.clone(),
            inner,
        })?;
        Ok(Self::Output { path: path.clone() })
    }
}

impl Rollback for MkdirUndoFinish {
    type Output = MkdirOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}
