use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Finish;
use super::Op;
use super::Rollback;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MkdirOp {
    pub path: PathBuf,
    pub parents: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MkdirOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
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

impl<'lua> Rollback<Op<'lua>> for MkdirOp {
    #[inline]
    fn rollback(&self) -> Op<'lua> {
        todo!()
    }
}
