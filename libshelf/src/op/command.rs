pub use crate::spec::NonZeroExitBehavior;

use std::io;
use std::process::Command;

use super::{Finish, Rollback};

#[derive(Debug, Clone)]
pub struct CommandOp {
    pub cmd: Command,
    pub nonzero_exit: NonZeroExitBehavior,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CommandOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

impl Finish for CommandOp {
    /// Return the command exit code.
    type Output = Option<i32>;
    type Error = CommandOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
        let Self {
            cmd,
            nonzero_exit: _,
        } = self;

        /// Spawn the command and wait for it to finish.
        let mut child = cmd.spawn()?;
        let res = child.wait()?;

        Ok(res.code())
    }
}

impl Rollback for CommandOp {
    #[inline]
    fn rollback(&self) -> Self {
        todo!()
    }
}
