use std::fs;
use std::io;
use std::path::PathBuf;

use super::Finish;

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
        if parents {
            fs::create_dir_all(&self.path)
        } else {
            fs::create_dir(&self.path)
        }

        // sl_error!("{$red}Couldn't create parent directories at{/$} {[green]} {$red}:{/$} {}", parent.absd(), err);
    }
}
