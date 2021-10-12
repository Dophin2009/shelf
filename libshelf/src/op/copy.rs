use std::fs;
use std::io;
use std::path::PathBuf;

use super::{Finish, ShouldFinish};

#[derive(Debug, Clone)]
pub struct CopyOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for CopyOp {
    type Output = ();
    type Error = io::Error;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        let _ = fs::copy(src, dest)?;
        Ok(())
    }
}

impl ShouldFinish for CopyOp {
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        let Self { src: _, dest } = self;
        Ok(dest.exists())
    }
}
