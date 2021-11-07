use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum RmOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmOp {
    pub path: PathBuf,
    pub dir: bool,
}

impl Finish for RmOp {
    type Output = ();
    type Error = RmOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path, dir } = self;
        let res = if *dir {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        Ok(res?)
    }
}

impl OpRollback for RmOp {
    type Output = RmUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, dir } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmUndoOp {
    pub path: PathBuf,
    pub dir: bool,
}

impl Finish for RmUndoOp {
    type Output = ();
    type Error = RmOpError;

    // FIXME: Pass in a saved directory and implement.
    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path: _, dir: _ } = self;

        unimplemented!();
    }
}

impl OpRollback for RmUndoOp {
    type Output = RmOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path, dir } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
        }
    }
}
