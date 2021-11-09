pub mod journal;

mod command;
mod copy;
mod create;
mod function;
mod link;
mod mkdir;
mod rm;
mod write;

pub(super) use crate::journal::Rollback;

pub use self::command::*;
pub use self::copy::*;
pub use self::create::*;
pub use self::function::*;
pub use self::link::*;
pub use self::mkdir::*;
pub use self::rm::*;
pub use self::write::*;

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

trait Finish {
    type Output;
    type Error;

    fn finish(&self) -> Result<Self::Output, Self::Error>;
}

/// Wrapper trait for [`Rollback`] in case we ever want to add more parameters.
pub trait OpRollback {
    type Output;

    /// Constructing the rollback operation should be deterministic and not perform any i/o
    /// operations.
    fn op_rollback(&self) -> Self::Output;
}

impl<R> Rollback for R
where
    R: OpRollback,
{
    type Output = R::Output;

    #[inline]
    fn rollback(&self) -> Self::Output {
        self.op_rollback()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Op<'lua> {
    Link(LinkOp),
    LinkUndo(LinkUndoOp),
    Copy(CopyOp),
    CopyUndo(CopyUndoOp),
    Create(CreateOp),
    CreateUndo(CreateUndoOp),
    Write(WriteOp),
    WriteUndo(WriteUndoOp),
    Mkdir(MkdirOp),
    MkdirUndo(MkdirUndoOp),
    Rm(RmOp),
    RmUndo(RmUndoOp),
    Command(CommandOp),
    CommandUndo(CommandUndoOp),

    // FIXME: any ways around this?
    #[serde(skip)]
    Function(FunctionOp<'lua>),
    #[serde(skip)]
    FunctionUndo(FunctionUndoOp<'lua>),
}

/// Generate From, [`OpRollback`] implementations for [`Op`].
macro_rules! op_impls {
    ($($Variant:ident => $SubOp:ty),*) => {
        $(
            impl<'lua> From<$SubOp> for Op<'lua> {
                #[inline]
                fn from(op: $SubOp) -> Self {
                    Self::$Variant(op)
                }
            }
        )*

        impl<'lua> OpRollback for Op<'lua> {
            type Output = Op<'lua>;

            #[inline]
            fn op_rollback(&self) -> Self {
                match self {
                    $(
                        Op::$Variant(op) => op.op_rollback().into(),
                    )*
                }
            }
        }
    };
}

op_impls!(
    Link => LinkOp,
    LinkUndo => LinkUndoOp,
    Copy => CopyOp,
    CopyUndo => CopyUndoOp,
    Create => CreateOp,
    CreateUndo => CreateUndoOp,
    Write => WriteOp,
    WriteUndo => WriteUndoOp,
    Mkdir => MkdirOp,
    MkdirUndo => MkdirUndoOp,
    Rm => RmOp,
    RmUndo => RmUndoOp,
    Command => CommandOp,
    CommandUndo => CommandUndoOp,
    Function => FunctionOp<'lua>,
    FunctionUndo => FunctionUndoOp<'lua>
);

#[derive(Debug, Clone)]
pub enum OpOutput<'lua> {
    Command(Option<i32>),
    Function(Option<mlua::Value<'lua>>),
    None,
}

impl<'lua> From<()> for OpOutput<'lua> {
    #[inline]
    fn from(_: ()) -> Self {
        Self::None
    }
}

macro_rules! op_output_from {
    ($Variant:ident => $SubOp:ty) => {
        impl<'lua> From<$SubOp> for OpOutput<'lua> {
            #[inline]
            fn from(v: $SubOp) -> Self {
                Self::$Variant(v)
            }
        }
    };
    ($($Variant:ident => $SubOp:ty),*) => {
        $(op_output_from!($Variant => $SubOp);)*
    };
}

op_output_from!(
    Command => Option<i32>,
    Function => Option<mlua::Value<'lua>>
);

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("link op error")]
    Link(#[from] LinkOpError),
    #[error("copy op error")]
    Copy(#[from] CopyOpError),
    #[error("create op error")]
    Create(#[from] CreateOpError),
    #[error("write op error")]
    Write(#[from] WriteOpError),
    #[error("mkdir op error")]
    Mkdir(#[from] MkdirOpError),
    #[error("rm op error")]
    Rm(#[from] RmOpError),
    #[error("command op error")]
    Command(#[from] CommandOpError),
    #[error("function op error")]
    Function(#[from] FunctionOpError),
}

impl<'lua> Finish for Op<'lua> {
    type Output = OpOutput<'lua>;
    type Error = OpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let res = match self {
            Op::Link(op) => op.finish()?.into(),
            Op::LinkUndo(op) => op.finish()?.into(),
            Op::Copy(op) => op.finish()?.into(),
            Op::CopyUndo(op) => op.finish()?.into(),
            Op::Create(op) => op.finish()?.into(),
            Op::CreateUndo(op) => op.finish()?.into(),
            Op::Write(op) => op.finish()?.into(),
            Op::WriteUndo(op) => op.finish()?.into(),
            Op::Mkdir(op) => op.finish()?.into(),
            Op::MkdirUndo(op) => op.finish()?.into(),
            Op::Rm(op) => op.finish()?.into(),
            Op::RmUndo(op) => op.finish()?.into(),
            Op::Command(op) => op.finish()?.into(),
            Op::CommandUndo(op) => op.finish()?.into(),
            Op::Function(op) => op.finish()?.into(),
            Op::FunctionUndo(op) => op.finish()?.into(),
        };

        Ok(res)
    }
}
