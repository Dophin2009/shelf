use std::fs;
use std::io;
use std::path::PathBuf;

use super::Finish;

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
        fs::copy(&self.src, &self.dest).map(|_| ())
    }
}
