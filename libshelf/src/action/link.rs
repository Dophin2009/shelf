use std::fs;
use std::path::{Path, PathBuf};

use crate::fsutil;
use crate::op::{CopyOp, LinkOp, Op, RmOp};

use super::{
    DoneOutput, InfoNotice, MkdirAction, Notice, Resolution, Resolve, ResolveOpts, SkipReason,
    WarnNotice,
};

#[derive(Debug, Clone)]
pub struct LinkAction {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub copy: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LinkActionError {
    #[error("missing file")]
    FileMissing(#[from] FileMissingError),
}

impl Resolve for LinkAction {
    type Error = LinkActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution, LinkActionError> {
        let Self {
            src,
            dest,
            copy,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (optional, fsutil::exists(src)) {
            (true, false) => {
                return Ok(Resolution::Skip(SkipReason::OptionalMissing {
                    path: src.clone(),
                }));
            }
            (false, false) => {
                return Err(Self::Error::FileMissing { path: src.clone() });
            }
            _ => {}
        };

        if copy {
            self.resolve_copy(opts)
        } else {
            self.resolve_link(opts)
        }
    }
}

impl LinkAction {
    // FIXME implement missing pieces
    #[inline]
    fn resolve_link(&self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut output = DoneOutput::empty();

        match fs::symlink_metadata(dest) {
            // For symlinks, check the target. If it's the same as src, then we should do nothing.
            Ok(meta) if meta.is_symlink() => {
                let target = fs::read_link(dest)?;
                if target == src {
                    return Ok(Resolution::Skip(SkipReason::DestinationExists {
                        path: path.clone(),
                    }));
                }
            }
            // For files and directories, warn about an overwrite, remove the file, and then link.
            Ok(meta) if meta.is_dir() || meta.is_file() => {
                output
                    .notices
                    .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                let dir = meta.is_dir();
                output.ops.push(Op::Rm(RmOp {
                    path: dest.clone(),
                    dir,
                }));
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Some(_) | Err(_) => {}
        };

        // Check for existence of parent directories and add op to make parent directories if they
        // don't exist.
        if let (mkparents_op) = mkparents_op(dest) {
            output.ops.push(mkparents_op);
        }

        output.ops.push(Op::Link(LinkOp {
            src: src.clone(),
            dest: dest.clone(),
        }));

        Ok(Resolution::Done(output))
    }

    #[inline]
    fn resolve_copy(&self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut output = DoneOutput::empty();

        match fs::symlink_metadata(dest) {
            // FIXME: For files, check the contents. If they match, we should do nothing.
            Ok(meta) if meta.is_file() => {
                output
                    .notices
                    .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                output.ops.push(Op::Rm(RmOp {
                    path: dest.clone(),
                    dir: false,
                }));

                // let content_same = match fs::read_to_string(dest) {
                // Ok(dest_contents) => dest_contents == contents,
                // // If error, just assume content is different
                // Err(_) => false,
                // };

                // if content_same {
                // return Ok(Resolution::Skip(SkipReason::DestinationExists {
                // path: path.clone(),
                // }));
                // }
            }
            // For directories and symlinks, warn about an overwrite, remove the directory, and then
            // link.
            Ok(meta) if meta.is_dir() | meta.is_symlink() => {
                output
                    .notices
                    .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                let dir = meta.is_dir();
                output.ops.push(Op::Rm(RmOp {
                    path: dest.clone(),
                    dir,
                }));
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Ok(_) | Err(_) => {}
        };

        // Check for existence of parent directories and add op to make parent directories if
        // they don't exist.
        if let (mkparents_op) = mkparents_op(dest) {
            output.ops.push(mkparents_op);
        }

        output.ops.push(Op::Copy(CopyOp {
            src: src.clone(),
            dest: dest.clone(),
        }));

        Ok(Resolution::Done(output))
    }
}

// FIXME: consider rollback behavior for this action...
#[inline]
pub(super) fn mkparents_op<P>(
    path: P,
    opts: &ResolveOpts,
) -> Result<Resolution, <MkdirAction as Resolve>::Error>
where
    P: AsRef<Path>,
{
    match path.as_ref().parent() {
        Some(parent) if !fsutil::exists(parent) => MkdirAction {
            path: path.to_path_buf(),
            parents: true,
        }
        .resolve(opts),
        _ => None,
    }
}
