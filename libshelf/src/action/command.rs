pub use crate::spec::{EnvMap, NonZeroExitBehavior};

use std::path::PathBuf;

use crate::fsutil;
use crate::op::{CommandOp, Op};

use super::error::FileMissingError;
use super::{Done, Res, Resolve, ResolveOpts};

#[derive(Debug, Clone)]
pub struct CommandAction {
    pub command: String,

    pub start: PathBuf,
    pub shell: String,

    // TODO: Use these
    pub stdout: bool,
    pub stderr: bool,

    pub clean_env: bool,
    pub env: EnvMap,

    pub nonzero_exit: NonZeroExitBehavior,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CommandActionError {
    #[error("start directory missing")]
    FileMissing(#[from] FileMissingError),
}

impl<'lua> Resolve<'lua> for CommandAction {
    type Error = CommandActionError;

    #[inline]
    fn resolve(&self, _opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let Self {
            command,
            start,
            shell,
            stdout: _,
            stderr: _,
            clean_env,
            env,
            nonzero_exit: _,
        } = self;

        if !fsutil::exists(start) {
            return Err(CommandActionError::FileMissing(FileMissingError {
                path: start.clone(),
            }));
        }

        let ops = vec![Op::Command(CommandOp {
            command: command.clone(),
            start: start.clone(),
            shell: shell.clone(),
            clean_env: *clean_env,
            env: env.clone(),
        })];
        Ok(Res::Done(Done {
            ops,
            notices: vec![],
        }))
    }
}
