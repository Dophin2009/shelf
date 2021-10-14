use std::path::PathBuf;
use std::{fs, io};

use serde::{Deserialize, Serialize};

use super::{Finish, Op, Rollback};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WriteOp {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WriteOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
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

impl<'lua> Rollback<Op<'lua>> for WriteOp {
    #[inline]
    fn rollback(&self) -> Op<'lua> {
        todo!()
    }
}
