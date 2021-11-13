use std::fs::{self, File};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::{CreateError, RemoveError};
use super::{Finish, Rollback};

sa::assert_impl_all!(CreateOp: Finish<Output = CreateFinish, Error = CreateOpError>);
sa::assert_impl_all!(CreateFinish: Rollback<Output = CreateUndoOp>);
sa::assert_impl_all!(CreateUndoOp: Finish<Output = CreateUndoFinish, Error = CreateOpError>);
sa::assert_impl_all!(CreateUndoFinish: Rollback<Output = CreateOp>);

/// Error encountered when finishing [`CreateOp`] or [`CreateUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum CreateOpError {
    #[error("create error")]
    Create(#[from] CreateError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
}

/// Operation to create a regular file at `path`.
///
/// # Errors
///
/// The operation will truncate existing data if `path` points to an existing file or error if
/// `path` is not writable.
///
/// # Undo
///
/// Undoing will remove the created file. This set of operations functions in the following cycle:
///
/// [`CreateOp`] --> [`CreateFinish`] --> [`CreateUndoOp`] --> [`CreateUndoFinish`] --> [`CreateOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateOp {
    /// Path of the file to be created.
    pub path: PathBuf,
}

/// The output of [`CreateOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateFinish {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

impl Finish for CreateOp {
    type Output = CreateFinish;
    type Error = CreateOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        // Create the file.
        let _ = File::create(path).map_err(|inner| CreateError {
            path: path.clone(),
            inner,
        })?;

        Ok(Self::Output { path: path.clone() })
    }
}

impl Rollback for CreateFinish {
    type Output = CreateUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}

/// The undo of [`CreateOp`] (see its documentation), created by rolling back [`CreateFinish`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateUndoOp {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

/// The output of [`CreateUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreateUndoFinish {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

impl Finish for CreateUndoOp {
    type Output = CreateUndoFinish;
    type Error = CreateOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        // Remove the created file.
        fs::remove_file(&path).map_err(|inner| RemoveError {
            path: path.clone(),
            inner,
        })?;

        Ok(Self::Output { path: path.clone() })
    }
}

impl Rollback for CreateUndoFinish {
    type Output = CreateOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}
