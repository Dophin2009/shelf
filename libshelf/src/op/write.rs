use std::path::PathBuf;
use std::{fs, io};

use super::{Finish, Rollback};

#[derive(Debug, Clone)]
pub struct WriteOp {
    path: PathBuf,
    contents: String,
}

#[derive(Debug, Clone, thiserror::Error)]
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

impl Rollback for WriteOp {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
