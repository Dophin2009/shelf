use std::fs;
use std::io;
use std::path::PathBuf;

use super::{Finish, ShouldFinish};

#[derive(Debug, Clone)]
pub struct LinkOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for LinkOp {
    type Output = ();
    type Error = io::Error;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = self.symlink()?;
        Ok(res)
    }
}

impl ShouldFinish for LinkOp {
    // Returns false if the destination file is a symlink that points to the same target as `src`
    // and true otherwise.
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        let Self { src, dest } = self;

        if dest.exists() {
            let target = fs::read_link(&dest)?;
            return Ok(target == src);
        } else {
            return Ok(true);
        }
    }
}

impl LinkOp {
    #[cfg(unix)]
    #[inline]
    fn symlink(&self) -> io::Result<()> {
        use std::os::unix;

        let Self { src, dest } = self;
        unix::fs::symlink(src, dest)
    }

    #[cfg(windows)]
    #[inline]
    fn symlink(&self) -> io::Result<()> {
        // FIXME: implement
        unimplemented!()
    }
}
