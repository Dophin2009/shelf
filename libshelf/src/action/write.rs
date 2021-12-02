use std::fs;
use std::path::PathBuf;

use crate::op::{CreateOp, MkdirOp, RmOp, WriteOp};

use super::{mkdir, Resolve};

/// Action to write `contents` to a file at `dest`.
#[derive(Debug, Clone)]
pub struct WriteAction {
    /// Path of the write destination.
    pub dest: PathBuf,
    /// Contents to be written.
    // TODO: AsRef<[u8]> instead?
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum Res {
    Normal(Vec<Op>),
    /// The existing destination file's contents will be overwritten.
    OverwriteContents(Vec<Op>),
    /// The existing destination file will be replaced.
    OverwriteFile(Vec<Op>),
    /// The action is skipped.
    Skip(Skip),
}

#[derive(Debug, Clone)]
pub enum Op {
    /// Remove operation.
    Rm(RmOp),
    /// Create operation.
    Create(CreateOp),
    /// Write operation.
    Write(WriteOp),
    /// Mkdir operation.
    Mkdir(MkdirOp),
}

/// Reason for skipping [`WriteAction`].
#[derive(Debug, Clone)]
pub enum Skip {
    /// Destination link already exists.
    DestExists(PathBuf),
}

impl Resolve for WriteAction {
    type Output = Res;

    #[inline]
    fn resolve(&self) -> Self::Output {
        let Self { dest, contents } = self;

        // If the destination file already exists, check the filetype.
        match fs::symlink_metadata(dest) {
            // For files, check the contents. If they match, we should do nothing.
            // Otherwise, warn about an overwrite and write.
            Ok(meta) if meta.is_file() => match fs::read(dest) {
                // Check for content same.
                Ok(dest_contents) if dest_contents == *contents => {
                    Res::Skip(Skip::DestExists(dest.clone()))
                }
                // If error, just assume content is different.
                Ok(_) | Err(_) => {
                    let ops = vec![self.as_op()];
                    Res::OverwriteContents(ops)
                }
            },

            // For other kinds of files, warn about an overwrite, remove the directory, create a
            // file, and then write.
            Ok(meta) if meta.is_dir() | meta.is_symlink() => {
                let dir = meta.is_dir();
                let ops = vec![
                    Op::Rm(RmOp {
                        path: dest.clone(),
                        dir,
                    }),
                    Op::Create(CreateOp { path: dest.clone() }),
                    self.as_op(),
                ];
                Res::OverwriteFile(ops)
            }

            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Ok(_) | Err(_) => {
                let mut ops = Vec::new();
                mkdir::mkdir_parents_ops(dest, &mut ops);
                let mut ops: Vec<_> = ops
                    .into_iter()
                    .map(|mkdir_op| Op::Mkdir(mkdir_op))
                    .collect();

                // We need to first create a file before writing to it.
                ops.push(Op::Create(CreateOp { path: dest.clone() }));
                // Add write operation.
                ops.push(self.as_op());

                Res::Normal(ops)
            }
        }
    }
}

impl WriteAction {
    #[inline]
    fn as_op(&self) -> Op {
        let Self { dest, contents } = self;

        Op::Write(WriteOp {
            path: dest.clone(),
            contents: contents.clone(),
        })
    }
}
