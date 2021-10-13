pub use crate::spec::{EnvMap, NonZeroExitBehavior};

use std::path::PathBuf;

use crate::fsutil;
use crate::op::{CommandOp, Op};

use super::error::FileMissingError;
use super::{DoneOutput, Resolution, Resolve, ResolveOpts};

#[derive(Debug, Clone)]
pub struct CommandAction {
    pub command: String,

    pub start: PathBuf,
    pub shell: String,

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

impl Resolve for CommandAction {
    type Error = CommandActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'_>, Self::Error> {
        let Self {
            command,
            start,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit,
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
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            clean_env: *clean_env,
            env: env.clone(),
            nonzero_exit: *nonzero_exit,
        })];
        Ok(Resolution::Done(DoneOutput {
            ops,
            notices: vec![],
        }))
    }
}
