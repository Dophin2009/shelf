use std::path::PathBuf;

use mlua::Function;

use crate::fsutil;
use crate::op::FunctionOp;

use super::error::FileMissingError;
use super::Resolve;

#[derive(Debug, Clone)]
pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,

    pub start: PathBuf,
}

#[derive(Debug, Clone)]
pub enum Res<'lua> {
    Normal(Vec<Op<'lua>>),
}

#[derive(Debug, Clone)]
pub enum Op<'lua> {
    /// Function op.
    Function(FunctionOp<'lua>),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("start directory missing")]
    StartMissing(#[from] FileMissingError),
}

impl<'lua> Resolve for FunctionAction<'lua> {
    type Output = Result<Res<'lua>, Error>;

    #[inline]
    fn resolve(&self) -> Self::Output {
        let Self { function, start } = self;

        // If the start directory doesn't exist, we should error.
        if fsutil::symlink_exists(start) {
            let ops = vec![Op::Function(FunctionOp {
                function: function.clone(),
                start: start.clone(),
            })];

            Ok(Res::Normal(ops))
        } else {
            Err(Error::StartMissing(FileMissingError {
                path: start.clone(),
            }))
        }
    }
}
