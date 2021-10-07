use std::fmt::Debug;
use std::fs;
use std::io;

pub trait Finish {
    type Output;
    type Error;

    fn finish(&self) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone)]
pub enum Op {
    Link(LinkOp),
    Copy(CopyOp),
    Mkdir(MkdirOp),
    Rm(RmOp),
}

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
