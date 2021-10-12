pub mod journal;

mod copy;
mod link;
mod mkdir;
mod rm;
mod write;

pub use self::copy::*;
pub use self::link::*;
pub use self::mkdir::*;
pub use self::rm::*;
pub use self::write::*;
pub(super) use crate::journal::Rollback;

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::journal::{self, Journal, JournalError, Record};

trait Finish {
    type Output;
    type Error;

    fn finish(&self) -> Result<Self::Output, Self::Error>;
}

trait ShouldFinish: Finish {
    fn should_finish(&self) -> Result<bool, Self::Error>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Op {
    Link(LinkOp),
    Copy(CopyOp),
    Write(WriteOp),
    Mkdir(MkdirOp),
    Rm(RmOp),
}

impl Rollback for Op {
    #[inline]
    fn rollback(&self) -> Self {
        match self {
            Op::Link(op) => op.rollback(),
            Op::Copy(op) => op.rollback(),
            Op::Write(op) => op.rollback(),
            Op::Mkdir(op) => op.rollback(),
            Op::Rm(op) => op.rollback(),
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

impl Finish for Op {
    type Output = OpOutput;
    type Error = OpError;

    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = match self {
            Op::Link(op) => op.finish(),
            Op::Copy(op) => op.finish(),
            Op::Write(op) => op.finish(),
            Op::Mkdir(op) => op.finish(),
            Op::Rm(op) => op.finish(),
        };

        let res = res?.into();
        Ok(res)
    }
}

impl ShouldFinish for Op {
    #[inline]
    fn should_finish(&self) -> Result<bool, Self::Error> {
        match op {
            Op::Link(op) => op.should_finish(),
            Op::Copy(op) => op.should_finish(),
            Op::Write(op) => op.should_finish(),
            Op::Mkdir(op) => op.should_finish(),
            Op::Rm(op) => op.should_finish(),
        }
    }
}
