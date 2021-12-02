use std::path::PathBuf;

use crate::fsutil;
use crate::op::CommandOp;

use super::error::FileMissingError;
use super::Resolve;

// Re-export action member types.
pub use crate::op::command::EnvMap;

#[derive(Debug, Clone)]
pub struct CommandAction {
    pub command: String,

    pub start: PathBuf,
    pub shell: String,

    pub clean_env: bool,
    pub env: EnvMap,
}

#[derive(Debug, Clone)]
pub enum Res {
    Normal(Vec<Op>),
}

#[derive(Debug, Clone)]
pub enum Op {
    /// Command op.
    Command(CommandOp),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("start directory missing")]
    StartMissing(#[from] FileMissingError),
}

impl Resolve for CommandAction {
    type Output = Result<Res, Error>;

    #[inline]
    fn resolve(&self) -> Self::Output {
        let Self {
            command,
            start,
            shell,
            clean_env,
            env,
        } = self;

        if fsutil::symlink_exists(start) {
            let ops = vec![Op::Command(CommandOp {
                command: command.clone(),
                start: start.clone(),
                shell: shell.clone(),
                clean_env: *clean_env,
                env: env.clone(),
            })];
            Ok(Res::Normal(ops))
        } else {
            Err(Error::StartMissing(FileMissingError {
                path: start.clone(),
            }))
        }
    }
}
