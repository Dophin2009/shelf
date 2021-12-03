use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::{OpenError, ReadError, WriteError};
use super::{Finish, Rollback};

sa::assert_impl_all!(WriteOp: Finish<Output = WriteFinish, Error = WriteOpError>);
sa::assert_impl_all!(WriteFinish: Rollback<Output = WriteUndoOp>);
sa::assert_impl_all!(WriteUndoOp: Finish<Output = WriteUndoFinish, Error = WriteOpError>);
sa::assert_impl_all!(WriteUndoFinish: Rollback<Output = WriteOp>);

/// Error encountered when finishing [`WriteOp`] or [`WriteUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum WriteOpError {
    #[error("open error")]
    Open(#[from] OpenError),
    #[error("read error")]
    Read(#[from] ReadError),
    #[error("write error")]
    Write(#[from] WriteError),
}

/// Operation to overwite the contents of `path` with `contents`.
///
/// # Errors
///
/// The file must already exist, or the operation will fail with no data being written.
///
/// # Undo
///
/// Undoing will restore the original contents. This set of operations functions in the following
/// cycle:
///
/// [`WriteOp`] --> [`WriteFinish`] --> [`WriteUndoOp`] --> [`WriteUndoFinish`] --> [`WriteOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WriteOp {
    /// Path of the file.
    pub path: PathBuf,
    /// Contents to be written to the file.
    pub contents: Vec<u8>,
}

/// The output of [`WriteOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WriteFinish {
    /// See [`WriteOp`].
    pub path: PathBuf,
    /// See [`WriteOp`].
    pub contents: Vec<u8>,

    /// Saved buffer of overwritten content.
    pub overwritten: Vec<u8>,
}

impl Finish for WriteOp {
    type Output = WriteFinish;
    type Error = WriteOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path, contents } = self;

        let mut overwritten = Vec::new();
        read_write_swap(path, contents, &mut overwritten)?;

        Ok(Self::Output {
            path: path.clone(),
            contents: contents.clone(),
            overwritten,
        })
    }
}

impl Rollback for WriteFinish {
    type Output = WriteUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self {
            path,
            contents,
            overwritten,
        } = self;

        Self::Output {
            path: path.clone(),
            contents: contents.clone(),
            overwritten: overwritten.clone(),
        }
    }
}

/// The undo of [`WriteOp`] (see its documentation), created by rolling back [`WriteFinish`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WriteUndoOp {
    /// See [`WriteOp`].
    pub path: PathBuf,
    /// See [`WriteOp`].
    pub contents: Vec<u8>,

    /// See [`WriteFinish`].
    pub overwritten: Vec<u8>,
}

/// The output of [`WriteUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WriteUndoFinish {
    /// See [`WriteOp`].
    pub path: PathBuf,
    /// See [`WriteOp`].
    pub contents: Vec<u8>,
}

impl Finish for WriteUndoOp {
    type Output = WriteUndoFinish;
    type Error = WriteOpError;

    // FIXME: We need a parameter that allows access to the original file contents.
    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self {
            path,
            contents,
            overwritten,
        } = self;

        read_write_swap(path, overwritten, &mut Vec::new())?;

        Ok(Self::Output {
            path: path.clone(),
            contents: contents.clone(),
        })
    }
}

impl Rollback for WriteUndoFinish {
    type Output = WriteOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path, contents } = self;

        Self::Output {
            path: path.clone(),
            contents: contents.clone(),
        }
    }
}

/// Open the file at `path`, read the contents into `overwritten`, and write `contents` to the
/// file.
#[inline]
fn read_write_swap<P>(
    path: P,
    contents: &[u8],
    overwritten: &mut Vec<u8>,
) -> Result<(), WriteOpError>
where
    P: AsRef<Path>,
{
    // Open file.
    let mut file = File::open(&path).map_err(|inner| OpenError {
        path: path.as_ref().to_path_buf(),
        inner,
    })?;

    // Save overwritten contents.
    file.read_to_end(overwritten).map_err(|inner| ReadError {
        path: path.as_ref().to_path_buf(),
        inner,
    })?;

    // Ovewrite contents.
    file.write_all(contents).map_err(|inner| WriteError {
        path: path.as_ref().to_path_buf(),
        inner,
    })?;

    Ok(())
}
