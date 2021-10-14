use std::fs::File;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, Op, Rollback};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateOp {
    pub path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
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

impl<'lua> Rollback<Op<'lua>> for CreateOp {
    #[inline]
    fn rollback(&self) -> Op<'lua> {
        todo!()
    }
}
