use std::path::{Path, PathBuf};
use std::{fs, io};

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::{
    CopyError, MetadataError, MkdirError, MoveError, ReadLinkError, RemoveError, RenameError,
    SymlinkError,
};
use super::{Finish, Rollback};

sa::assert_impl_all!(RmOp: Finish<Output = RmFinish, Error = RmOpError>);
sa::assert_impl_all!(RmFinish: Rollback<Output = RmUndoOp>);
sa::assert_impl_all!(RmUndoOp: Finish<Output = RmUndoFinish, Error = RmUndoOpError>);
sa::assert_impl_all!(RmUndoFinish: Rollback<Output = RmOp>);

/// Error encountered when finishing [`RmOp`].
#[derive(Debug, thiserror::Error)]
pub enum RmOpError {
    #[error("rename error")]
    Rename(#[from] RenameError),
    #[error("symlink read error")]
    ReadLink(#[from] ReadLinkError),
    #[error("metadata error")]
    Metadata(#[from] MetadataError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
    #[error("symlink error")]
    Symlink(#[from] SymlinkError),
    #[error("move error")]
    Move(#[from] MoveError),
    #[error("copy error")]
    Copy(#[from] CopyError),
    #[error("mkdir error")]
    Mkdir(#[from] MkdirError),
}

/// Operation to remove a file or directory at `path`.
///
/// # Errors
///
/// The operation will error if there is no existing file at `path` or there are insufficient
/// permissions. If there is a directory at `path`, but `dir` is false, an error will occur.
///
/// # Undo
///
/// Undoing will restore the file or directory (and its contents). This set of operations functions
/// in the following cycle:
///
/// [`RmOp`] --> [`RmFinish`] --> [`RmUndoOp`] --> [`RmUndoFinish`] --> [`RmOp`] --> ...
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmOp {
    /// Path at which the file or directory will be removed.
    pub path: PathBuf,
    /// If true, finishing will try to delete a directory. This must be set to `true` if `path` is
    /// a directory.
    pub dir: bool,
}

/// The output of [`RmOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,

    /// Path at which the file was backed-up.
    pub safepath: PathBuf,
}

impl Finish for RmOp {
    type Output = RmFinish;
    type Error = RmOpError;

    #[inline]
    fn finish(&self, ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self { path, dir } = self;

        let safepath = ctx.filesafe.resolve(path);
        rename_with_fallback(path, &safepath)?;

        Ok(Self::Output {
            path: path.clone(),
            dir: *dir,
            safepath,
        })
    }
}

impl Rollback for RmFinish {
    type Output = RmUndoOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self {
            path,
            dir,
            safepath,
        } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
            safepath: safepath.clone(),
        }
    }
}

/// Error encountered when finishing [`RmUndoOp`].
#[derive(Debug, thiserror::Error)]
pub enum RmUndoOpError {
    #[error("rename error")]
    Rename(#[from] RenameError),
    #[error("symlink read error")]
    ReadLink(#[from] ReadLinkError),
    #[error("metadata error")]
    Metadata(#[from] MetadataError),
    #[error("remove error")]
    Remove(#[from] RemoveError),
    #[error("symlink error")]
    Symlink(#[from] SymlinkError),
    #[error("move error")]
    Move(#[from] MoveError),
    #[error("copy error")]
    Copy(#[from] CopyError),
    #[error("mkdir error")]
    Mkdir(#[from] MkdirError),
}

/// The undo of [`RmOp`] (see its documentation), created by rolling back [`RmFinish`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmUndoOp {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,

    /// See [`RmFinish`].
    pub safepath: PathBuf,
}

/// The output of [`RmUndoOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RmUndoFinish {
    /// See [`RmOp`].
    pub path: PathBuf,
    /// See [`RmOp`].
    pub dir: bool,
}

impl Finish for RmUndoOp {
    type Output = RmUndoFinish;
    type Error = RmUndoOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self {
            path,
            dir,
            safepath,
        } = self;

        match rename_with_fallback(path, safepath) {
            Ok(()) => {}
            Err(err) => match err {
                RmOpError::Rename(err) => return Err(err.into()),
                RmOpError::ReadLink(err) => return Err(err.into()),
                RmOpError::Metadata(err) => return Err(err.into()),
                RmOpError::Remove(err) => return Err(err.into()),
                RmOpError::Symlink(err) => return Err(err.into()),
                RmOpError::Move(err) => return Err(err.into()),
                RmOpError::Copy(err) => return Err(err.into()),
                RmOpError::Mkdir(err) => return Err(err.into()),
            },
        }

        Ok(Self::Output {
            path: path.clone(),
            dir: *dir,
        })
    }
}

impl Rollback for RmUndoFinish {
    type Output = RmOp;

    #[inline]
    fn rollback(&self) -> Self::Output {
        let Self { path, dir } = self;

        Self::Output {
            path: path.clone(),
            dir: *dir,
        }
    }
}

#[inline]
fn rename_with_fallback(path: &Path, safepath: &Path) -> Result<(), RmOpError> {
    // TODO: Try to lift detection of filesystem up to action level?
    // First try a rename, then fallback to copying and removing.
    if fs::rename(path, &safepath).is_err() {
        let metadata = path.symlink_metadata().map_err(|err| MetadataError {
            path: path.to_path_buf(),
            inner: err,
        })?;
        let ft = metadata.file_type();

        if let Some(parent) = safepath.parent() {
            fs::create_dir_all(parent).map_err(|err| MkdirError {
                path: parent.to_path_buf(),
                inner: err,
            })?;
        }

        if ft.is_symlink() {
            let target = fs::read_link(path).map_err(|err| ReadLinkError {
                path: path.to_path_buf(),
                inner: err,
            })?;

            #[cfg(not(any(unix, windows)))]
            {
                compile_error!("This platform doesn't support symlinks")
            }

            let symlink_res = {
                #[cfg(unix)]
                {
                    use std::os::unix;
                    unix::fs::symlink(&target, &safepath)
                }

                #[cfg(windows)]
                {
                    use std::os::windows;

                    let is_dir = target.is_dir();
                    let symlink_res = if target.exists() && is_dir {
                        windows::fs::symlink_dir(&target, &safepath)
                    } else {
                        windows::fs::symlink_file(&target, &safepath)
                    };
                }
            };

            symlink_res.map_err(|err| SymlinkError {
                src: target.clone(),
                dest: safepath.to_path_buf(),
                inner: err,
            })?;

            fs::remove_file(&path).map_err(|err| RemoveError {
                path: path.to_path_buf(),
                inner: err,
            })?;
        } else if ft.is_dir() {
            if safepath.exists() {
                let remove_res = if safepath.is_dir() {
                    fs::remove_dir_all(&safepath)
                } else {
                    fs::remove_file(&safepath)
                };
                remove_res.map_err(|err| RemoveError {
                    path: safepath.to_path_buf(),
                    inner: err,
                })?
            }

            let opts = fs_extra::dir::CopyOptions {
                copy_inside: true,
                ..Default::default()
            };
            if let Err(err) = fs_extra::dir::move_dir(path, &safepath, &opts) {
                return Err(RmOpError::Move(MoveError {
                    src: path.to_path_buf(),
                    dest: safepath.to_path_buf(),
                    inner: io::Error::new(io::ErrorKind::Other, format!("{}", err)),
                }));
            }
        } else {
            fs::copy(path, &safepath).map_err(|err| CopyError {
                src: path.to_path_buf(),
                dest: safepath.to_path_buf(),
                inner: err,
            })?;
            fs::remove_file(path).map_err(|err| RemoveError {
                path: path.to_path_buf(),
                inner: err,
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::fse;

    use super::super::test;
    use super::{Finish, RmOp, Rollback};

    /// Test regular file.
    #[test]
    fn test_regular_file() -> test::Result<()> {
        test::with_tempdir(|dir, ctx| {
            let (_, path) = test::new_file(dir, "a")?;

            let op = RmOp {
                path: path.clone(),
                dir: false,
            };

            let opf = op.finish(ctx)?;
            assert!(!fse::symlink_exists(&path));
            assert!(fse::symlink_exists(&opf.safepath));

            let undo = opf.rollback();
            let undof = undo.finish(ctx)?;
            assert!(fse::symlink_exists(&path));
            assert!(!fse::symlink_exists(&opf.safepath));

            let op2 = undof.rollback();
            assert_eq!(op, op2);

            Ok(())
        })
    }

    /// Test for nonexistent file.
    #[test]
    fn test_nonexistent_file() -> test::Result<()> {
        test::with_tempdir(|dir, ctx| {
            let path = dir.join("a");
            let op = RmOp { path, dir: false };

            if op.finish(ctx).is_ok() {
                panic!("op succeeded")
            }

            Ok(())
        })
    }
}
