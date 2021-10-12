pub use crate::spec::NonZeroExitBehavior;

use std::env;
use std::path::PathBuf;

use mlua::Function;

use crate::action::FunctionActionError;

#[derive(Debug, Clone)]
pub struct FunctionOp<'lua> {
    function: Function<'lua>,
    start: PathBuf,
    nonzero_exit: NonZeroExitBehavior,
}

#[derive(Debug, Clone)]
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
            function,
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
        let ret: mlua::Value = function.call(())?;
        match ret {
            mlua::Value::Nil => None,
            v => Some(v),
        }
    }
}

impl<'lua> Rollback for FunctionOp<'lua> {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
