use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum CreateOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateOp {
    pub path: PathBuf,
}

impl Finish for CreateOp {
    type Output = ();
    type Error = CreateOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        File::create(path)?;
        Ok(())
    }
}

impl OpRollback for CreateOp {
    type Output = CreateUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { path } = self;

        Self::Output { path: path.clone() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateUndoOp {
    pub path: PathBuf,
}

impl Finish for CreateUndoOp {
    type Output = ();
    type Error = CreateOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path } = self;

        fs::remove_dir(&path)?;
        Ok(())
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
