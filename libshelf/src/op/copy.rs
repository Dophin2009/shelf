use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::{CopyError, RemoveError};
use super::{Finish, Rollback};

sa::assert_impl_all!(CopyOp: Finish<Output = CopyFinish, Error = CopyOpError>);
sa::assert_impl_all!(CopyFinish: Rollback<Output = CopyUndoOp>);
sa::assert_impl_all!(CopyUndoOp: Finish<Output = CopyUndoFinish, Error = CopyOpError>);
sa::assert_impl_all!(CopyUndoFinish: Rollback<Output = CopyOp>);

/// Error encountered when finishing [`CopyOp`] or [`CopyUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum CopyOpError {
    #[error("copy error")]
    Copy(#[from] CopyError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
}

/// Operation to copy a file from `src` to `dest`.
///
/// In the case that `src` and `dest` are the same path, the file will likely be truncated (see
/// [`fs::copy`]).
///
/// # Errors
///
/// It is assumed that `src` points to an readable regular file and symlink, and that no file
/// exists at `dest` (which must be writable). These premises are not checked, and the operation
/// will error if they are not met.
///
/// # Undo
///
/// Undoing will delete the copied file. This set of operations functions in the following cycle:
///
/// [`CopyOp`] --> [`CopyFinish`] --> [`CopyUndoOp`] --> [`CopyUndoFinish`] --> [`CopyOp`] --> ...
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyOp {
    /// Path to file to copy.
    pub src: PathBuf,
    /// Path to destination of copy.
    pub dest: PathBuf,
}

/// The output of [`CopyOp`]. See its documentation for information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyFinish {
    /// See [`CopyOp`].
    pub src: PathBuf,
    /// See [`CopyOp`].
    pub dest: PathBuf,
}

impl Finish for CopyOp {
    type Output = CopyFinish;
    type Error = CopyOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        // Perform copy.
        let _ = fs::copy(src, dest).map_err(|inner| CopyError {
            src: src.clone(),
            dest: dest.clone(),
            inner,
        })?;

        Ok(Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        })
    }
}

impl Rollback for CopyFinish {
    type Output = CopyUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}

/// The undo of [`CopyOp`] (see its documentation), created by rolling back [`CopyFinish`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyUndoOp {
    /// See [`CopyOp`].
    pub src: PathBuf,
    /// See [`CopyOp`].
    pub dest: PathBuf,
}

/// The output of [`CopyUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyUndoFinish {
    /// See [`CopyOp`].
    pub src: PathBuf,
    /// See [`CopyOp`].
    pub dest: PathBuf,
}

impl Finish for CopyUndoOp {
    type Output = CopyUndoFinish;
    type Error = CopyOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        // Remove copied file.
        let _ = fs::remove_file(dest).map_err(|inner| RemoveError {
            path: dest.clone(),
            inner,
        })?;

        Ok(Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        })
    }
}

impl Rollback for CopyUndoFinish {
    type Output = CopyOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}
