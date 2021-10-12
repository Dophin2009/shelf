use std::fs;
use std::path::{Path, PathBuf};

use super::{
    DoneOutput, InfoNotice, Notice, Resolution, ResolutionError, Resolve, ResolveOpts,
    ResolveResult, SkipReason, WarnNotice,
};
use crate::op::{CopyOp, LinkOp, Op, RmOp};

pub struct LinkAction {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub copy: bool,
    pub optional: bool,
}

impl Resolve for LinkAction {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> ResolveResult {
        let Self {
            src,
            dest,
            copy,
            optional,
        } = self;

        // If file does not exist and optional flag enabled, skip.
        // If optional flag disabled, error.
        match (optional, src.exists()) {
            (true, false) => {
                // log_skip!("{[green]} does not exist", src.reld());
                return Ok(Resolution::Skip(SkipReason::OptionalMissing { path: src }));
            }
            (false, false) => {
                // log_miss(&src);
                return Err(ResolutionError::FileMissing { path: src });
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
    fn resolve_link(self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        let mut do_link = true;

        let dest_meta = match fs::symlink_metadata(&dest) {
            Ok(meta) => {
                if meta.is_file() {
                    // For files, warn about an overwrite, remove the file, and then link.
                    notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    ops.push(Op::Rm(RmOp {
                        path: &dest,
                        dir: false,
                    }));
                } else if meta.is_dir() {
                    // For directories, warn about an overwrite, remove the directory, and then
                    // link.
                    notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    ops.push(Op::Rm(RmOp {
                        path: &dest,
                        dir: true,
                    }));
                } else if meta.is_symlink() {
                    // For symlinks, check the target. If it's the same as src, then we should do
                    // nothing.
                    let target = fs::read_link(&dest)?;
                    if target == src {
                        do_link = false;
                    }
                }
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Err(err) => {}
        };

        if do_link {
            // FIXME: consider rollback behavior for this action...
            // Check for existence of parent directories and add op to make parent directories if
            // they don't exist.
            if let (mkparents_op) = Self::mkparents_op(dest) {
                ops.push(mkparents_op);
            }

            ops.push(Op::Link(LinkOp { src, dest }));
        }

        Ok(Resolution::Done(DoneOutput { ops, notices }))
    }

    #[inline]
    fn resolve_copy(self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        let mut do_copy = true;

        let dest_meta = match fs::symlink_metadata(&dest) {
            Ok(meta) => {
                if meta.is_file() {
                    notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    ops.push(Op::Rm(RmOp {
                        path: &dest,
                        dir: false,
                    }));

                    // FIXME: For files, check the contents. If they match, we should do nothing.
                    // let content_same = match fs::read_to_string(&dest) {
                    // Ok(dest_contents) => dest_contents == contents,
                    // // If error, just assume content is different
                    // Err(_) => false,
                    // };

                    // if content_same {
                    // do_copy = false;
                    // }
                } else if meta.is_dir() {
                    // For directories, warn about an overwrite, remove the directory, and then
                    // link.
                    notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    ops.push(Op::Rm(RmOp {
                        path: &dest,
                        dir: true,
                    }));
                } else if meta.is_symlink() {
                    // For symlinks, warn about an overwrite, remove the file, and then copy.
                    notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    ops.push(Op::Rm(RmOp {
                        path: &dest,
                        dir: false,
                    }));
                }
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Err(err) => {}
        };

        if do_copy {
            // FIXME: consider rollback behavior for this action...
            // Check for existence of parent directories and add op to make parent directories if
            // they don't exist.
            if let (mkparents_op) = Self::mkparents_op(dest) {
                ops.push(mkparents_op);
            }

            ops.push(Op::Copy(CopyOp { src, dest }));
        }

        Ok(Resolution::Done(DoneOutput { ops, notices }))
    }

    #[inline]
    fn mkparents_op<P>(path: P) -> Option<Op>
    where
        P: AsRef<Path>,
    {
        match path.as_ref().parent() {
            Some(parent) if !parent.exists() => Some(Op::Mkdir(MkdirOp {
                path: parent.to_path_buf(),
                parents: true,
            })),
            _ => None,
        }
    }
}
