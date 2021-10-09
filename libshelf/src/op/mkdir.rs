use std::fs;
use std::io;
use std::path::PathBuf;

use super::{Finish, ShouldFinish};

#[derive(Debug, Clone)]
pub struct MkdirOp {
    pub path: PathBuf,
    pub parents: bool,
}

impl Finish for MkdirOp {
    type Output = ();
    type Error = io::Error;

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

impl ShouldFinish for MkdirOp {
    /// Returns true if the target path doesn't exist.
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        let Self { path, parents: _ } = self;

        Ok(!self.path.exists())
    }
}
