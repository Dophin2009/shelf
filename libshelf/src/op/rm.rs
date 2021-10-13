use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Finish;
use super::Rollback;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RmOp {
    pub path: PathBuf,
    pub dir: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum RmOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

impl Finish for RmOp {
    type Output = ();
    type Error = RmOpError;

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

impl Rollback for RmOp {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
