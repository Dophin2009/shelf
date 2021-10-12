use std::fs;
use std::path::PathBuf;

use crate::fsutil;
use crate::op::{Op, RmOp};

use super::{link, DoneOutput, Notice, Resolution, Resolve, WarnNotice};

#[derive(Debug, Clone)]
pub struct WriteAction {
    pub dest: PathBuf,
    pub contents: String,
}

impl Resolve for WriteAction {
    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> ResolveResult {
        let Self { dest, contents } = self;

        let mut output = DoneOutput::empty();
        let mut do_write = true;

        // If the destination file already exists, check the filetype.
        match fs::symlink_metadata(dest) {
            Ok(meta) => {
                if meta.is_file() {
                    output
                        .notices
                        .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    output.ops.push(Op::Rm(RmOp {
                        path: dest.clone(),
                        dir: false,
                    }));

                    // FIXME: For files, check the contents. If they match, we should do nothing.
                    // let content_same = match fs::read_to_string(dest) {
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
                    output
                        .notices
                        .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    output.ops.push(Op::Rm(RmOp {
                        path: dest.clone(),
                        dir: true,
                    }));
                } else if meta.is_symlink() {
                    // For symlinks, warn about an overwrite, remove the file, and then copy.
                    output
                        .notices
                        .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));
                    output.ops.push(Op::Rm(RmOp {
                        path: dest.clone(),
                        dir: false,
                    }));
                }
            }
            // File doesn't exist, or insufficient permissions; treat as nonexistent.
            Err(_) => {}
        };

        if do_write {
            // Check for existence of parent directories and add op to make parent directories if
            // they don't exist.
            if let (mkparents_op) = link::mkparents_op(dest) {
                output.ops.push(mkparents_op);
            }

            output.ops.push(Op::Write(WriteOp {
                path: dest.clone(),
                contents: contents.clone(),
            }));
        }

        Ok(Resolution::Done(output))
    }
}
