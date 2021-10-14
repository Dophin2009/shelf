pub use crate::spec::{EnvMap, NonZeroExitBehavior};

use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use super::{Finish, Op, Rollback};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandOp {
    pub command: String,

    pub start: PathBuf,
    pub shell: String,

    pub stdout: bool,
    pub stderr: bool,

    pub clean_env: bool,
    pub env: EnvMap,

    pub nonzero_exit: NonZeroExitBehavior,
}

#[derive(Debug, thiserror::Error)]
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
            command,
            start,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit: _,
        } = self;

        let mut cmd = Command::new(shell);
        cmd.args(&["-c", &command]);

        cmd.current_dir(start);

        if !stdout {
            cmd.stdout(Stdio::null());
        }
        if !stderr {
            cmd.stderr(Stdio::null());
        }

        if *clean_env {
            cmd.env_clear();
        }

        if !env.is_empty() {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        // Spawn the command and wait for it to finish.
        let mut child = cmd.spawn()?;
        let res = child.wait()?;

        Ok(res.code())
    }
}

impl<'lua> Rollback<Op<'lua>> for CommandOp {
    #[inline]
    fn rollback(&self) -> Op<'lua> {
        todo!()
    }
}
