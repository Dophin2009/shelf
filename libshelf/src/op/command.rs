use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::ctx::FinishCtx;
use super::error::SpawnError;
use super::Finish;

sa::assert_impl_all!(CommandOp: Finish<Output = CommandFinish, Error = CommandOpError>);

/// Error encountered when finishing [`CommandOp`].
#[derive(Debug, thiserror::Error)]
pub enum CommandOpError {
    #[error("spawn error")]
    Spawn(#[from] SpawnError),
}

/// Map of environment variables and values.
pub type EnvMap = HashMap<String, String>;

/// Operation to run a shell command.
///
/// # Errors
///
/// See [`Command`].
///
/// # Undo
///
/// This operation is not undo-able.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, )]
pub struct CommandOp {
    /// Shell command to run.
    pub command: String,
    /// Initial directory in which the command will be run.
    pub start: PathBuf,
    /// Shell to use (e.g. sh or bash).
    pub shell: String,

    /// If true, all environment variables will be cleared before execution.
    pub clean_env: bool,
    /// Map of extra environment variables to set.
    pub env: EnvMap,
}

/// The output of [`CommandOp`]. See its documentation for information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFinish {
    /// Shell command to run.
    pub command: String,
    /// Initial directory in which the command will be run.
    pub start: PathBuf,
    /// Shell to use (e.g. sh or bash).
    pub shell: String,

    /// If true, all environment variables will be cleared before execution.
    pub clean_env: bool,
    /// Map of extra environment variables to set.
    pub env: EnvMap,

    /// Output of the command.
    pub output: Output,
}

impl Finish for CommandOp {
    /// Return the command exit code.
    type Output = CommandFinish;
    type Error = CommandOpError;

    #[inline]
    fn finish(&self, _ctx: &FinishCtx) -> Result<Self::Output, Self::Error> {
        let Self {
            command,
            start,
            shell,
            clean_env,
            env,
        } = self;

        let mut cmd = Command::new(shell);
        // TODO: Don't hardcode this
        cmd.args(&["-c", command]);
        cmd.current_dir(start);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if *clean_env {
            cmd.env_clear();
        }

        if !env.is_empty() {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let output = self.spawn_output(cmd).map_err(|inner| SpawnError {
            command: command.clone(),
            shell: shell.clone(),
            start: start.clone(),
            inner,
        })?;

        Ok(Self::Output {
            command: command.clone(),
            start: start.clone(),
            shell: shell.clone(),
            clean_env: *clean_env,
            env: env.clone(),
            output,
        })
    }
}

impl CommandOp {
    /// Execute a command and wait for it to finish.
    #[inline]
    fn spawn_output(&self, mut cmd: Command) -> Result<Output, io::Error> {
        let child = cmd.spawn()?;
        child.wait_with_output()
    }
}
