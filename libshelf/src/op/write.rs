use std::path::PathBuf;
use std::{fs, io};

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum WriteOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

/// A write operation overwites the contents of a file. The file must already exist, or the
/// operation will fail with no data being written.
///
/// A rollback will restore the original contents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WriteOp {
    pub path: PathBuf,
    pub contents: String,
}

impl Finish for WriteOp {
    type Output = ();
    type Error = WriteOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path, contents } = self;

        fs::write(path, contents)?;
        Ok(())
    }
}

impl OpRollback for WriteOp {
    type Output = WriteUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, contents } = self;

        Self::Output {
            path: path.clone(),
            contents: contents.clone(),
        }
    }
}

/// The rollback of a [`WriteOp`].
///
/// See its documentation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WriteUndoOp {
    pub path: PathBuf,
    pub contents: String,
}

impl Finish for WriteUndoOp {
    type Output = ();
    type Error = WriteOpError;

    // FIXME: We need a parameter that allows access to the original file contents.
    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self {
            path: _,
            contents: _,
        } = self;

        unimplemented!();
    }
}

impl OpRollback for WriteUndoOp {
    type Output = WriteOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, contents } = self;

        Self::Output {
            path: path.clone(),
            contents: contents.clone(),
        }
    }
}
