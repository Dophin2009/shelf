pub use crate::spec::NonZeroExitBehavior;

use std::fmt;
use std::path::PathBuf;

use mlua::Function;

use crate::fsutil;
use crate::op::{FunctionOp, Op};

use super::error::FileMissingError;
use super::{DoneOutput, Resolution, Resolve, ResolveOpts};

#[derive(Clone)]
pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,

    pub start: PathBuf,
    pub nonzero_exit: NonZeroExitBehavior,
}

impl<'lua> fmt::Debug for FunctionAction<'lua> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionAction")
            .field("function", &"<lua function>")
            .field("start", &self.start)
            .field("error_exit", &self.nonzero_exit)
            .finish()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FunctionActionError {
    #[error("start directory missing")]
    FileMissing(#[from] FileMissingError),
}

impl<'lua> Resolve<'lua> for FunctionAction<'lua> {
    type Error = FunctionActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'lua>, Self::Error> {
        let Self {
            function,
            start,
            nonzero_exit,
        } = self;

        // If the start directory doesn't exist, we should error.
        if !fsutil::exists(start) {
            return Err(FunctionActionError::FileMissing(FileMissingError {
                path: start.clone(),
            }));
        }

        let ops = vec![Op::Function(FunctionOp {
            function: function.clone(),
            start: start.clone(),
            nonzero_exit: *nonzero_exit,
        })];
        Ok(Resolution::Done(DoneOutput {
            ops,
            notices: vec![],
        }))
    }
}
