use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum CopyOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for CopyOp {
    type Output = ();
    type Error = CopyOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        let _ = fs::copy(src, dest)?;
        Ok(())
    }
}

impl OpRollback for CopyOp {
    type Output = CopyUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { src, dest } = self;
        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyUndoOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for CopyUndoOp {
    type Output = ();
    type Error = CopyOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { src: _, dest } = self;

        let _ = fs::remove_dir_all(dest)?;
        Ok(())
    }
}

impl OpRollback for CopyUndoOp {
    type Output = CopyOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}
