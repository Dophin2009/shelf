use std::fs;
use std::io;
use std::path::PathBuf;

use super::Finish;

#[derive(Debug, Clone)]
pub struct RmOp {
    pub path: PathBuf,
    pub dir: bool,
}

impl Finish for RmOp {
    type Output = ();
    type Error = io::Error;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        if dir {
            fs::remove_dir_all(&self.dest)
        } else {
            fs::remove_file(&self.dest)
        }

        // sl_error!("{$red}Couldn't delete the symlink:{/$} {}", err);
        // sl_i_error!("{$red}Destination:{/$} {[green]}", dest.absd());

        // sl_error!("{$red}Couldn't delete the file:{/$} {}", err);
        // sl_i_error!("{$yellow}Location:{/$} {[green]}", dest.absd());

        // sl_error!("{$red}Couldn't delete the directory:{/$} {}", err);
        // sl_i_error!("{$yellow}Location:{/$} {[green]}", dest.absd());
    }
}
