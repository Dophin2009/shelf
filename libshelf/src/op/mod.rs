pub mod journal;

mod command;
mod copy;
mod function;
mod link;
mod mkdir;
mod rm;
mod write;

pub(super) use crate::journal::Rollback;

pub use self::command::*;
pub use self::copy::*;
pub use self::function::*;
pub use self::link::*;
pub use self::mkdir::*;
pub use self::rm::*;
pub use self::write::*;

use std::fmt::Debug;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};

trait Finish {
    type Output;
    type Error;

    fn finish(&self) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Op<'lua> {
    Link(LinkOp),
    Copy(CopyOp),
    Write(WriteOp),
    Mkdir(MkdirOp),
    Rm(RmOp),
    Command(CommandOp),
    Function(FunctionOp<'lua>),
}

impl<'lua> Rollback for Op<'lua> {
    #[inline]
    fn rollback(&self) -> Self {
        match self {
            Op::Link(op) => op.rollback(),
            Op::Copy(op) => op.rollback(),
            Op::Write(op) => op.rollback(),
            Op::Mkdir(op) => op.rollback(),
            Op::Rm(op) => op.rollback(),
            Op::Command(op) => op.rollback(),
            Op::Function(op) => op.rollback(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OpOutput {
    None,
}

impl From<()> for OpOutput {
    #[inline]
    fn from(_: ()) -> Self {
        Self::None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

impl<'lua> Finish for Op<'lua> {
    type Output = OpOutput;
    type Error = OpError;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = match self {
            Op::Link(op) => op.finish(),
            Op::Copy(op) => op.finish(),
            Op::Write(op) => op.finish(),
            Op::Mkdir(op) => op.finish(),
            Op::Rm(op) => op.finish(),
            Op::Command(op) => op.finish(),
            Op::Function(op) => op.finish(),
        };

        let res = res?.into();
        Ok(res)
    }
}
