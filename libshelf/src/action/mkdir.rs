use std::fs;
use std::path::PathBuf;

use crate::action::SkipReason;
use crate::op::{MkdirOp, Op, RmOp};

use super::error::NoError;
use super::{DoneOutput, Notice, Resolution, Resolve, ResolveOpts, WarnNotice};

#[derive(Debug, Clone)]
pub struct MkdirAction {
    pub path: PathBuf,
    pub parents: bool,
}

pub type MkdirActionError = NoError;

impl<'lua> Resolve<'lua> for MkdirAction {
    type Error = MkdirActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'lua>, Self::Error> {
        let Self { path, parents } = self;

        let mut output = DoneOutput::empty();

        match fs::symlink_metadata(path) {
            // For directories, we should do nothing, as it already exists.
            Ok(meta) if meta.is_dir() => {
                return Ok(Resolution::Skip(SkipReason::DestinationExists {
                    path: path.clone(),
                }));
            }
            // For files and symlinks, warn about an overwrite, remove the file, and then link.
            Ok(meta) if meta.is_file() || meta.is_symlink() => {
                output
                    .notices
                    .push(Notice::Warn(WarnNotice::Overwrite { path: path.clone() }));
                output.ops.push(Op::Rm(RmOp {
                    path: path.clone(),
                    dir: false,
                }));
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Ok(_) | Err(_) => {}
        };

        // Add op to mkdir.
        output.ops.push(Op::Mkdir(MkdirOp {
            path: path.clone(),
            parents: *parents,
        }));

        Ok(Resolution::Done(output))
    }
}
