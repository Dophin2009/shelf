pub use crate::spec::NonZeroExitBehavior;

use std::env;
use std::fmt;
use std::path::PathBuf;

use mlua::Function;

use super::{Finish, OpRollback};

#[derive(Debug, Clone, thiserror::Error)]
pub enum FunctionOpError {
    #[error("lua error")]
    Lua(#[from] mlua::Error),
}

#[derive(Clone)]
pub struct FunctionOp<'lua> {
    pub function: Function<'lua>,
    pub start: PathBuf,
    pub nonzero_exit: NonZeroExitBehavior,
}

impl<'lua> fmt::Debug for FunctionOp<'lua> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOp")
            .field("function", &"<lua function>")
            .field("start", &self.start)
            .field("nonzero_exit", &self.nonzero_exit)
            .finish()
    }
}

impl<'lua> Finish for FunctionOp<'lua> {
    type Output = Option<mlua::Value<'lua>>;
    type Error = FunctionOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self {
            function: _,
            start,
            nonzero_exit: _,
        } = self;

        // Change to the start directory.
        let cwd = env::current_dir().unwrap();
        env::set_current_dir(start).unwrap();

        // Call the function.
        let ret = self.call();

        // Restore cwd regardless of error or not.
        env::set_current_dir(&cwd).unwrap();

        ret
    }
}

impl<'lua> FunctionOp<'lua> {
    #[inline]
    fn call(&self) -> Result<Option<mlua::Value<'lua>>, FunctionOpError> {
        let ret: mlua::Value = self.function.call(())?;
        let ret = match ret {
            mlua::Value::Nil => None,
            v => Some(v),
        };

        Ok(ret)
    }
}

impl<'lua> OpRollback for FunctionOp<'lua> {
    type Output = FunctionUndoOp<'lua>;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self {
            function,
            start,
            nonzero_exit,
        } = self;

        Self::Output {
            function: function.clone(),
            start: start.clone(),
            nonzero_exit: *nonzero_exit,
        }
    }
}

#[derive(Clone)]
pub struct FunctionUndoOp<'lua> {
    pub function: Function<'lua>,
    pub start: PathBuf,
    pub nonzero_exit: NonZeroExitBehavior,
}

impl<'lua> fmt::Debug for FunctionUndoOp<'lua> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOp")
            .field("function", &"<lua function>")
            .field("start", &self.start)
            .field("nonzero_exit", &self.nonzero_exit)
            .finish()
    }
}

impl<'lua> Finish for FunctionUndoOp<'lua> {
    type Output = ();
    type Error = FunctionOpError;

    // FIXME: How to rollback a function call??
    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        todo!()
    }
}

impl<'lua> OpRollback for FunctionUndoOp<'lua> {
    type Output = FunctionOp<'lua>;

    #[inline]
    fn op_rollback(&self) -> Self::Output {
        let Self {
            function,
            start,
            nonzero_exit,
        } = self;

        Self::Output {
            function: function.clone(),
            start: start.clone(),
            nonzero_exit: *nonzero_exit,
        }
    }
}
