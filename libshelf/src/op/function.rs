pub use crate::spec::NonZeroExitBehavior;

use std::env;
use std::fmt;
use std::path::PathBuf;

use mlua::Function;

use super::{Finish, Rollback};

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

#[derive(Debug, Clone, thiserror::Error)]
pub enum FunctionOpError {
    #[error("lua error")]
    Lua(#[from] mlua::Error),
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

impl<'lua> Rollback for FunctionOp<'lua> {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
