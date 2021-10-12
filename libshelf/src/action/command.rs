use crate::fsutil;
pub use crate::spec::{EnvMap, NonZeroExitBehavior};

use std::path::PathBuf;
use std::process::Command;

use crate::op::{CommandOp, Op};

use super::error::FileMissingError;
use super::{DoneOutput, Resolve};

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
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution, Self::Error> {
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

        let mut cmd = Command::new(shell);
        cmd.args(&["-c", &command]);

        if fsutil::exists(start) {
            cmd.current_dir(start);
        } else {
            return Err(CommandActionError::FileMissing(FileMissingError {
                path: start.clone(),
            }));
        }

        if !stdout {
            cmd.stdout(Stdio::null());
        }
        if !stderr {
            cmd.stderr(Stdio::null());
        }

        if clean_env {
            cmd.env_clear();
        }

        if !env.is_empty() {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let ops = vec![Op::Command(CommandOp { cmd, nonzero_exit })];
        Ok(Resolution::Done(DoneOutput {
            ops,
            notices: vec![],
        }))
    }
}
