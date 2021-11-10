use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::{Finish, OpRollback};

sa::assert_impl_all!(LinkOp: Finish<Output = LinkFinish, Error = LinkOpError>);
sa::assert_impl_all!(LinkFinish: OpRollback<Output = LinkUndoOp>);
sa::assert_impl_all!(LinkUndoOp: Finish<Output = LinkUndoFinish, Error = LinkOpError>);
sa::assert_impl_all!(LinkUndoFinish: OpRollback<Output = LinkOp>);

/// Error encountered when finishing [`LinkOp`] or [`LinkUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum LinkOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

/// Operation to link a file from `src` to `dest`. It roughly corresponds to
/// [`std::unix::fs::symlink`] on Unix and (???) on Windows.
///
/// # Errors
///
/// It is assumed that `src` points to an readable file, and that no file exists at `dest` (which
/// must be writable). These premises are not checked, and the operation will error if they are not
/// met.
///
/// # Undo
///
/// Undoing will delete the symlink. This set of operations functions in the following cycle:
///
/// [`LinkOp`] --> [`LinkFinish`] --> [`LinkUndoOp`] --> [`LinkUndoFinish`] --> [`LinkOp`] --> ...
///
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkOp {
    /// Path to file to link.
    pub src: PathBuf,
    /// Path to destination of link.
    pub dest: PathBuf,
}

/// The output of [`LinkOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkFinish {
    /// See [`LinkOp`].
    pub src: PathBuf,
    /// See [`LinkOp`].
    pub dest: PathBuf,
}

impl Finish for LinkOp {
    type Output = LinkFinish;
    type Error = LinkOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        self.symlink()?;
        Ok(Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        })
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

impl OpRollback for LinkFinish {
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

/// The undo of [`LinkOp`] (see its documentation), created by rolling back [`LinkFinish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkUndoOp {
    /// See [`LinkOp`].
    pub src: PathBuf,
    /// See [`LinkOp`].
    pub dest: PathBuf,
}

/// The output of [`LinkUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkUndoFinish {
    /// See [`LinkOp`].
    pub src: PathBuf,
    /// See [`LinkOp`].
    pub dest: PathBuf,
}

impl Finish for LinkUndoOp {
    type Output = LinkUndoFinish;
    type Error = LinkOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { src: _, dest } = self;

        fs::remove_dir_all(dest)?;
        Ok(Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        })
    }
}

impl OpRollback for LinkUndoFinish {
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
