use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::{RemoveError, SymlinkError};
use super::{Finish, Rollback};

sa::assert_impl_all!(LinkOp: Finish<Output = LinkFinish, Error = LinkOpError>);
sa::assert_impl_all!(LinkFinish: Rollback<Output = LinkUndoOp>);
sa::assert_impl_all!(LinkUndoOp: Finish<Output = LinkUndoFinish, Error = LinkOpError>);
sa::assert_impl_all!(LinkUndoFinish: Rollback<Output = LinkOp>);

/// Error encountered when finishing [`LinkOp`] or [`LinkUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum LinkOpError {
    #[error("symlink error")]
    Symlink(#[from] SymlinkError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
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
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkOp {
    /// Path to file to link.
    pub src: PathBuf,
    /// Path to destination of link.
    pub dest: PathBuf,
}

/// The output of [`LinkOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
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
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        // Perform symlink.
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
    fn symlink(&self) -> Result<(), SymlinkError> {
        use std::os::unix;

        let Self { src, dest } = self;

        unix::fs::symlink(src, dest).map_err(|inner| SymlinkError {
            src: src.clone(),
            dest: dest.clone(),
            inner,
        })
    }

    #[cfg(windows)]
    #[inline]
    fn symlink(&self) -> Result<(), SymlinkError> {
        // FIXME: Look into Windows API behavior
        unimplemented!()
    }
}

impl Rollback for LinkFinish {
    type Output = LinkUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}

/// The undo of [`LinkOp`] (see its documentation), created by rolling back [`LinkFinish`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkUndoOp {
    /// See [`LinkOp`].
    pub src: PathBuf,
    /// See [`LinkOp`].
    pub dest: PathBuf,
}

/// The output of [`LinkUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
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
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { src, dest } = self;

        // Remove symlink.
        fs::remove_file(dest).map_err(|inner| RemoveError {
            path: dest.clone(),
            inner,
        })?;

        Ok(Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        })
    }
}

impl Rollback for LinkUndoFinish {
    type Output = LinkOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { src, dest } = self;

        Self::Output {
            src: src.clone(),
            dest: dest.clone(),
        }
    }
}
