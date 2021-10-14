use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Op;
use super::{Finish, Rollback};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CopyOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
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

impl<'lua> Rollback<Op<'lua>> for CopyOp {
    #[inline]
    fn rollback(&self) -> Op<'lua> {
        todo!()
    }
}
