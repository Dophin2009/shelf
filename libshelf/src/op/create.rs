use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use crate::fsutil;

use super::{Finish, OpRollback};

sa::assert_impl_all!(CreateOp: Finish<Output = CreateFinish, Error = CreateOpError>);
sa::assert_impl_all!(CreateFinish: OpRollback<Output = CreateUndoOp>);
sa::assert_impl_all!(CreateUndoOp: Finish<Output = CreateUndoFinish, Error = CreateOpError>);
sa::assert_impl_all!(CreateUndoFinish: OpRollback<Output = CreateOp>);

/// Error encountered when finishing [`CreateOp`] or [`CreateUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum CreateOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

/// Operation to create a regular file at `path`.
///
/// # Errors
///
/// The operation will error or truncate existing data if `path` points to an existing file or if
/// `path` is not writable.
///
/// # Undo
///
/// Undoing will remove the created file. This set of operations functions in the following cycle:
///
/// [`CreateOp`] --> [`CreateFinish`] --> [`CreateUndoOp`] --> [`CreateUndoFinish`] --> [`CreateOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateOp {
    /// Path of the file to be created.
    pub path: PathBuf,
}

/// The output of [`CreateOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateFinish {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

impl Finish for CreateOp {
    type Output = CreateFinish;
    type Error = CreateOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        // Create the file.
        let _ = File::create(path)?;
        Ok(Self::Output { path: path.clone() })
    }
}

impl OpRollback for CreateFinish {
    type Output = CreateUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}

/// The undo of [`CreateOp`] (see its documentation), created by rolling back [`CreateFinish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUndoOp {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

/// The output of [`CreateUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUndoFinish {
    /// See [`CreateOp`].
    pub path: PathBuf,
}

impl Finish for CreateUndoOp {
    type Output = CreateUndoFinish;
    type Error = CreateOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        fs::remove_dir(&path)?;
        Ok(Self::Output { path: path.clone() })
    }
}

impl OpRollback for CreateUndoOp {
    type Output = CreateOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}
