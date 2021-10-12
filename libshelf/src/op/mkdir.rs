use std::fs;
use std::io;
use std::path::PathBuf;

use super::Finish;

#[derive(Debug, Clone)]
pub struct MkdirOp {
    pub path: PathBuf,
    pub parents: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
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

        let res = if parents {
            fs::create_dir_all(path)
        } else {
            fs::create_dir(path)
        };
        Ok(res?)
    }
}
