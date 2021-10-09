use std::io;
use std::path::PathBuf;

use super::Finish;

#[derive(Debug, Clone)]
pub struct LinkOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for LinkOp {
    type Output = ();
    type Error = io::Error;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        self.symlink()

        // sl_error!("{$red}Couldn't symlink:{/$} {}", err);
        // sl_i_error!("Source: {[green]}",  src.absd());
        // sl_i_error!("Destination: {[green]}", dest.absd());
    }
}

impl LinkOp {
    #[cfg(unix)]
    #[inline]
    fn symlink(&self) -> io::Result<()> {
        use std::os::unix;
        unix::fs::symlink(&self.src, &self.dest)
    }

    #[cfg(windows)]
    #[inline]
    fn symlink(&self) -> io::Result<()> {
        // FIXME: implement
        unimplemented!()
    }
}
