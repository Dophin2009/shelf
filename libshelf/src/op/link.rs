use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Finish, OpRollback};

#[derive(Debug, thiserror::Error)]
pub enum LinkOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for LinkOp {
    type Output = ();
    type Error = LinkOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = self.symlink()?;
        Ok(res)
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
        // FIXME: Look into Windows API behavior
        unimplemented!()
    }
}

impl OpRollback for LinkOp {
    type Output = LinkUndoOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkUndoOp {
    pub src: PathBuf,
    pub dest: PathBuf,
}

impl Finish for LinkUndoOp {
    type Output = ();
    type Error = LinkOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { src: _, dest } = self;

        fs::remove_dir_all(dest)?;
        Ok(())
    }
}

impl OpRollback for LinkUndoOp {
    type Output = LinkOp;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}
