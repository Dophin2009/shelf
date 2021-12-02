use std::fs;
use std::path::{Path, PathBuf};

use crate::fse;
use crate::op::{MkdirOp, RmOp};

use super::Resolve;

#[derive(Debug, Clone)]
pub struct MkdirAction {
    /// Path at which to create directories.
    pub path: PathBuf,
    /// Missing parrents should also be created.
    pub parents: bool,
}

#[derive(Debug, Clone)]
pub enum Res {
    Normal(Vec<Op>),
    /// The destination file or directory will be overwritten.
    Overwrite(Vec<Op>),
    /// The action is skipped.
    Skip(Skip),
}

#[derive(Debug, Clone)]
pub enum Op {
    /// Remove operation.
    Rm(RmOp),
    /// Mkdir operation.
    Mkdir(MkdirOp),
}

/// Reason for skipping [`LinkAction`].
#[derive(Debug, Clone)]
pub enum Skip {
    /// Optional `src` does not exist.
    OptMissing(PathBuf),
    /// Destination link already exists.
    DestExists(PathBuf),
}

impl Resolve for MkdirAction {
    type Output = Res;

    #[inline]
    fn resolve(&self) -> Self::Output {
        let Self { path, parents } = self;

        let (overwrite, is_dir) = match fs::symlink_metadata(path) {
            // For directories, we should do nothing, as it already exists.
            Ok(meta) if meta.is_dir() => {
                return Res::Skip(Skip::DestExists(path.clone()));
            }

            // For files and symlinks, warn about an overwrite, remove the file, and then link.
            Ok(_) => (true, false),

            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Err(_) => (false, false),
        };

        if overwrite {
            Res::Overwrite(vec![Op::Rm(RmOp {
                path: path.clone(),
                dir: is_dir,
            })])
        } else {
            let mut ops = if *parents {
                let mut ops = Vec::new();
                mkdir_parents_ops(path, &mut ops);
                ops.into_iter().map(Op::Mkdir).collect()
            } else {
                Vec::new()
            };

            ops.push(Op::Mkdir(MkdirOp { path: path.clone() }));

            Res::Normal(ops)
        }
    }
}

#[inline]
pub fn mkdir_parents_ops<P>(path: P, ops: &mut Vec<MkdirOp>)
where
    P: AsRef<Path>,
{
    let mut parent_opt = Some(path.as_ref());

    while let Some(parent) = parent_opt.as_ref() {
        // Add mkdir ops for all nonexisting parents.
        if !fse::symlink_exists(parent) {
            ops.push(MkdirOp {
                path: parent.to_path_buf(),
            });
        }

        parent_opt = parent.parent();
    }
}
