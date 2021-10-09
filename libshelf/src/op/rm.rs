use std::fs;
use std::io;
use std::path::PathBuf;

use super::{Finish, ShouldFinish};

#[derive(Debug, Clone)]
pub struct RmOp {
    pub path: PathBuf,
    pub dir: bool,
}

impl Finish for RmOp {
    type Output = ();
    type Error = io::Error;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { path, dir } = self;
        let res = if dir {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        Ok(res?)
    }
}

impl ShouldFinish for RmOp {
    /// Returns true if a file or directory exists at the target path.
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        let Self { path, dir: _ } = self;
        Ok(path.exists())
    }
}
