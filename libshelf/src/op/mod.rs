pub mod ctx;
pub mod journal;

pub mod error;

pub mod command;
pub mod copy;
pub mod create;
pub mod function;
pub mod link;
pub mod mkdir;
pub mod rm;
pub mod write;

pub(super) use crate::journal::Rollback;

pub use self::{
    command::CommandOp,
    copy::{CopyOp, CopyUndoOp},
    create::{CreateOp, CreateUndoOp},
    function::FunctionOp,
    link::{LinkOp, LinkUndoOp},
    mkdir::{MkdirOp, MkdirUndoOp},
    rm::{RmOp, RmUndoOp},
    write::{WriteOp, WriteUndoOp},
};

use std::fmt::Debug;

use self::ctx::FinishCtx;

pub trait Finish {
    type Output;
    type Error;

    fn finish(&self, ctx: &FinishCtx) -> Result<Self::Output, Self::Error>;
}

/// The finish of an op.
///
/// # Example
///
/// Finished<LinkOp> == LinkFinish;
/// Finished<LinkUndoOp> == LinkUndoFinish;
pub type Finished<O> = <O as Finish>::Output;

/// The finish error type of an op.
///
/// # Example
///
/// FinishedError<LinkOp> == LinkOpError;
/// FinishedError<LinkUndoOp> == LinkOpError;
pub type FinishedError<O> = <O as Finish>::Error;

/// The rollback of a finish.
///
/// # Example
///
/// Rolledback<LinkFinish> == LinkUndoOp;
/// Rolledback<LinkUndoFinish> == LinkOp;
pub type Rolledback<F> = <F as Rollback>::Output;

/// The undo op is the rollback of the finish.
///
/// # Example
///
/// Undo<LinkOp> == Rolledback<LinkFinish> == LinkUndoOp;
/// Undo<LinkUndoOp> == Rolledback<LinkUndoFinish> == LinkOp;
pub type Undo<O> = Rolledback<Finished<O>>;

/// The finish of the undo of an op.
///
/// UndoFinished<LinkOp> == Finished<LinkUndoOp> == LinkUndoFinish;
/// UndoFinished<LinkUndoOp> == Finished<LinkOp> == LinkFinish;
pub type UndoFinished<O> = Finished<Undo<O>>;

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("link op error")]
    Link(#[from] FinishedError<LinkOp>),
    #[error("copy op error")]
    Copy(#[from] FinishedError<CopyOp>),
    #[error("create op error")]
    Create(#[from] FinishedError<CreateOp>),
    #[error("write op error")]
    Write(#[from] FinishedError<WriteOp>),
    #[error("mkdir op error")]
    Mkdir(#[from] FinishedError<MkdirOp>),
    #[error("rm op error")]
    Rm(#[from] FinishedError<RmOp>),
    #[error("command op error")]
    Command(#[from] FinishedError<CommandOp>),
    #[error("function op error")]
    Function(#[from] FinishedError<FunctionOp<'static>>),
}

#[derive(Debug, Clone)]
pub enum Op<'lua> {
    Link(LinkOp),
    LinkUndo(Undo<LinkOp>),
    Copy(CopyOp),
    CopyUndo(Undo<CopyOp>),
    Create(CreateOp),
    CreateUndo(Undo<CreateOp>),
    Write(WriteOp),
    WriteUndo(Undo<WriteOp>),
    Mkdir(MkdirOp),
    MkdirUndo(Undo<MkdirOp>),
    Rm(RmOp),
    RmUndo(Undo<RmOp>),
    Command(CommandOp),
    Function(FunctionOp<'lua>),
}

/// Some test utilities.
#[cfg(test)]
mod test {
    use std::fs::{File};
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::ctx::{FileSafe, FinishCtx};

    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    // Create a new file and return the path.
    pub fn new_file<P, Q>(dir: P, path: Q) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let path = dir.as_ref().join(path);
        File::create(&path)?;

        Ok(path)
    }

    pub fn with_tempdir<F>(f: F) -> Result<()>
    where
        F: Fn(&Path, &FinishCtx) -> Result<()>,
    {
        let (dir, ctx, _safedir) = ctx()?;
        f(dir.path(), &ctx)
    }

    fn ctx() -> Result<(TempDir, FinishCtx, TempDir)> {
        let dir = tempfile::tempdir()?;
        let safedir = tempfile::tempdir()?;

        let ctx = FinishCtx {
            filesafe: FileSafe::new(safedir.path()),
        };

        Ok((dir, ctx, safedir))
    }
}
