pub use crate::spec::{EnvMap, NonZeroExitBehavior};

use std::io;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde::{Deserialize, Serialize};
use static_assertions as sa;

use super::{Finish, OpRollback};

sa::assert_impl_all!(CommandOp: Finish<Output = CommandFinish, Error = CommandOpError>);

/// Error encountered when finishing [`CommandOp`].
#[derive(Debug, thiserror::Error)]
pub enum CommandOpError {
    #[error("i/o error")]
    Io(#[from] io::Error),
}

/// Operation to run a shell command.
///
/// # Errors
///
/// See [`Command`].
///
/// # Undo
///
/// This operation is not undo-able.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    // TODO: Move this type into this module
    pub env: EnvMap,
}

/// The output of [`CommandOp`]. See its documentation for information.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    // TODO: Move this type into this module
    pub env: EnvMap,

    /// Output of the command.
    pub output: Output,
}

impl Finish for CommandOp {
    /// Return the command exit code.
    type Output = CommandFinish;
    type Error = CommandOpError;

    #[inline]
    fn finish(&self) -> Result<Self::Output, Self::Error> {
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

        // Execute the command and wait for it to finish.
        let child = cmd.spawn()?;
        let output = child.wait_with_output()?;

        Ok(Self::Output {
            command: command.clone(),
            start: start.clone(),
            shell: shell.clone(),
            clean_env: *clean_env,
            env: env.clone(),
        })
    }
}
