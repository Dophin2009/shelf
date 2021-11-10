pub use crate::spec::NonZeroExitBehavior;

use std::env;
use std::fmt;
use std::path::PathBuf;

use mlua::Function;
use static_assertions as sa;

use super::Finish;

sa::assert_impl_all!(FunctionOp<'static>: Finish<Output = FunctionFinish<'static>, Error = FunctionOpError>);

/// Error encountered when finishing [`FunctionOp`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum FunctionOpError {
    #[error("lua error")]
    Lua(#[from] mlua::Error),
}

/// Operation to run a Lua function.
///
/// # Errors
///
/// See [`Function`].
///
/// # Undo
///
/// This operation is not undo-able.
#[derive(Clone)]
pub struct FunctionOp<'lua> {
    /// Handle to the Lua function to call.
    pub function: Function<'lua>,
    /// Initial directory in which the function will be called.
    pub start: PathBuf,
}

/// The output of [`FunctionOp`]. See its documentation for information.
#[derive(Clone)]
pub struct FunctionFinish<'lua> {
    /// See [`FunctionOp`].
    pub function: Function<'lua>,
    /// See [`FunctionOp`].
    pub start: PathBuf,

    /// The return value from the function call.
    pub ret: Option<mlua::Value<'lua>>,
}

impl<'lua> fmt::Debug for FunctionOp<'lua> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionOp")
            .field("function", &"<lua function>")
            .field("start", &self.start)
            .finish()
    }
}

impl<'lua> fmt::Debug for FunctionFinish<'lua> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionFinish")
            .field("function", &"<lua function>")
            .field("start", &self.start)
            .field("ret", &"<lua value>")
            .finish()
    }
}

impl<'lua> Finish for FunctionOp<'lua> {
    type Output = FunctionFinish<'lua>;
    type Error = FunctionOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self { function, start } = self;

        // Change to the start directory.
        let cwd = env::current_dir().unwrap();
        env::set_current_dir(start).unwrap();

        // Call the function.
        let ret = self.call();

        // Restore cwd regardless of error or not.
        env::set_current_dir(&cwd).unwrap();

        let ret = ret?;
        Ok(Self::Output {
            function: function.clone(),
            start: start.clone(),
            ret,
        })
    }
}

impl<'lua> FunctionOp<'lua> {
    /// Call the Lua function and return the return value.
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
