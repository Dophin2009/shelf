use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum MkdirOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirOp {
    pub path: PathBuf,
    pub parents: bool,
}

impl Finish for MkdirOp {
    type Output = ();
    type Error = MkdirOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path, parents } = self;

        let res = if *parents {
            fs::create_dir_all(path)
        } else {
            fs::create_dir(path)
        };
        Ok(res?)
    }
}

impl OpRollback for MkdirOp {
    type Output = MkdirUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, parents } = self;

        Self::Output {
            path: path.clone(),
            parents: *parents,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirUndoOp {
    pub path: PathBuf,
    pub parents: bool,
}

impl Finish for MkdirUndoOp {
    type Output = ();
    type Error = MkdirOpError;

    // FIXME: Rollback create_dir_all??
    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path, parents: _ } = self;

        fs::remove_dir_all(path)?;
        Ok(())
    }
}

impl OpRollback for MkdirUndoOp {
    type Output = MkdirOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, parents } = self;

        Self::Output {
            path: path.clone(),
            parents: *parents,
        }
    }
}
