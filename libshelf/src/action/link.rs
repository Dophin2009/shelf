use std::fs;
use std::path::{Path, PathBuf};

use super::{
    DoneOutput, InfoNotice, Notice, Resolution, ResolutionError, Resolve, ResolveOpts,
    ResolveResult, SkipReason, WarnNotice,
};
use crate::op::{Op, RmOp};

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
    fn resolve_copy(self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        let mut do_copy = true;

        // Check the cache for the destination path.
        let dest_cache = cache.get(&dest);
        match (dest.exists(), dest_cache) {
            // Destination file exists and is found in cache.
            // Check the kind of file that should exist there.
            // Remove from cache.
            (true, Some(dest_cache)) => match dest_cache.typ {
                // Existing should be a normal file.
                // Check actual file type.
                FileMetaTyp::File { content_hash } => match Self::read_filetype(&dest)? {
                    // For files, check their content hash.
                    FileType::File => {
                        // If the content hash doesn't match, emit a notice and remove from cache.
                        // If it does, do nothing.
                        unimplemented!()
                    }
                    // Actual file is a directory or link.
                    ft @ FileType::Dir | FileType::Link => {
                        // Emit a warning about overwriting.
                        // Remove data from cache.
                        // Add op to remove existing file.
                        Self::overwrite_sequence(
                            &dest,
                            Some(&ft),
                            &mut ops,
                            &mut notices,
                            Some(cache),
                        )?;
                    }
                },

                // Existing should be a directory or a sumlink.
                // Emit a warning about overwriting.
                FileMetaTyp::Dir | FileMetaTyp::Link => {
                    Self::overwrite_sequence(&dest, None, &mut ops, &mut notices, Some(cache))?;
                }
            },

            // Destination file exists, but is not found in cache.
            (true, None) => {
                // Emit a warning about overwriting.
                // Add op to remove existing file.
                Self::overwrite_sequence(&dest, None, &mut ops, &mut notices, None)?;
            }

            // Destination file doesn't exist, but is found in cache.
            (false, Some(dest_cache)) => {
                // Warn about changes that might've occurred since last run.
                notices.push(Notice::Warn(WarnNotice::ManualChange {
                    path: dest.clone(),
                }));

                // Remove data from cache.
                cache.remove(&dest);
            }

            // Destination file doesn't exist, and is not found in cache.
            // Carry on with copying.
            (false, None) => {}
        };

        if do_copy {
            // Check for existence of parent directories and add op to make parent directories if
            // they don't exist.
            if let (mkparents_op) = Self::mkparents_op(dest) {
                ops.push(mkparents_op);
            }

            // FIXME: cache this action

            ops.push(Op::Copy(CopyOp { src, dest }));
        }

        Ok(Resolution::Done(DoneOutput { ops, notices }))
    }

    #[inline]
    fn resolve_link(self, opts: &ResolveOpts) -> ResolveResult {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        let mut do_link = true;

        // sl_debug!("Found an existing file at the destination; checking...");

        // Check the cache for the destination path.
        let dest_cache = cache.get(&self.dest);
        match (dest.exists(), dest_cache) {
            // Destination file exists, and is found in cache.
            // Check the kind of file that should exist there.
            (true, Some()) => match dest_cache.typ {
                // Existing should be a normal file. Check if this is case.
                // Emit notice about overwriting.
                // Remove from cache.
                FileMetaTyp::File { .. } | FileMetaTyp::Dir => {
                    Self::overwrite_sequence(&dest, None, &mut ops, &mut notices, Some(cache))?;
                }
                // Existing is a symlink, check where it points.
                FileMetaTyp::Link => {
                    // Check if the actual file is a symlink.
                    match Self::read_filetype(&dest)? {
                        FileType::Link => {
                            // Destination file is a symlink.
                            // Read the target location and check against src.
                            let dest_target = match fs::read_link(&dest) {
                                Ok(path) => path,
                                Err(err) => {
                                    return Err(ResolutionError::FileReadMetadata {
                                        path: dest,
                                        err,
                                    });
                                    // sl_error!("{$red}Couldn't follow symlink:{/$} {}", err);
                                    // sl_i_error!("{$red}Location:{/$} {[green]}", dest.absd());
                                }
                            };

                            if dest_target == src {
                                Self::overwrite_sequence(
                                    &dest,
                                    None,
                                    &mut ops,
                                    &mut notices,
                                    Some(cache),
                                )?;
                                // Destination symlink target and src are the same. Emit a notice.
                                notices.push(Notice::Info(InfoNotice::ExistingSymlink {
                                    path: dest.clone(),
                                    target: dest_target,
                                }));

                                // Don't link.
                                do_link = false;

                                // sl_debug!("Symlink was already established; doing nothing...");
                            } else {
                                Self::overwrite_sequence(
                                    &dest,
                                    FileType::Link,
                                    &mut ops,
                                    &mut notices,
                                    Some(cache),
                                )?;

                                // sl_warn!("{$yellow}An existing symlink (pointing to a different location) was found at the destination{/$}");
                                // sl_i_warn!("{$yellow}Destination:{/$} {[green]}", dest.absd());
                                // sl_i_warn!("{$yellow}It will be replaced.{/$}");
                            }
                        }
                        ft @ FileType::Dir | FileType::File => {
                            // Destination file is not a symlink.
                            // Emit a warning about possible manual action.
                            notices.push(Notice::Warn(WarnNotice::ManualChange {
                                path: dest.clone(),
                            }));

                            // Emit a warning about overwriting.
                            notices
                                .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                            // Add op to remove the existing file.
                            let is_dir = ft == FileType::Dir;
                            ops.push(Op::Rm(RmOp {
                                path: dest,
                                dir: is_dir,
                            }));

                            // sl_warn!("{$yellow}An existing file or directory was found at the destination{/$}");
                            // sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
                            // sl_i_warn!("{$yellow}It will be replaced.{/$}");
                        }
                    }
                }
            },
            // Destination file exists, but is not found in cache.
            // Emit warning about overwrite.
            (true, None) => {
                notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                // Add op to remove the existing.
                let ft = Self::read_filetype(&dest)?;
                let is_dir = ft == FileType::Dir;
                ops.push(Op::Rm(RmOp {
                    path: dest.clone(),
                    dir: is_dir,
                }));
            }
            // Destination file doesn't exist, and is found in cache.
            // Emit warning about possible manual change.
            (false, Some()) => {
                notices.push(Notice::Warn(WarnNotice::ManualChange {
                    path: dest.clone(),
                }));

                // Remove data from cache.
                cache.remove(&dest);
            }
            // Destination file doesn't exist, and is not found in cache.
            // Carry on with linking.
            (false, None) => {}
        }

        // sl_debug!(
        // "Linking file: {[green]} to {[green]}",
        // src.reld(),
        // dest.reld()
        // );

        if do_link {
            if let (mkparents_op) = Self::mkparents_op(dest) {
                ops.push(mkparents_op);
            }
            ops.push(Op::Link(LinkOp { src, dest }));
        }

        Ok(Resolution::Done(DoneOutput { ops, notices }))
    }

    #[inline]
    fn overwrite_sequence<C>(
        dest: &PathBuf,
        ft: Option<&FileType>,
        ops: &mut Vec<Op>,
        notices: &mut Vec<Notice>,
        cache: Option<&mut C>,
    ) -> Result<(), ResolutionError>
    where
        C: Cache,
    {
        // Emit a warning about overwriting.
        notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

        // Remove data from cache.
        if let Some(cache) = cache {
            cache.remove(&dest);
        }

        // Add op to remove existing.
        let ft = ft
            .map(Result::Ok)
            .unwrap_or_else(|| Self::read_filetype(dest))?;
        let is_dir = ft == FileType::Dir;
        ops.push(Op::Rm(RmOp {
            path: dest.clone(),
            dir: is_dir,
        }));
    }

    #[inline]
    fn read_filetype<P>(path: P) -> Result<FileType, ResolutionError>
    where
        P: AsRef<Path>,
    {
        let meta = match fs::symlink_metadata(&dest) {
            Ok(meta) => meta,
            // TODO: not sure if this is the best behavior
            // If couldn't read metadata, return error.
            Err(err) => {
                return Err(ResolutionError::FileReadMetadata { path: dest, err });
                // sl_error!("{$red}Couldn't read file metadata:{/$} {}", err);
                // sl_i_error!("{$red}{/$}");
            }
        };

        // Check if the actual file is a symlink.
        let ft = meta.file_type();
        let ft = if ft.is_dir() {
            FileType::Dir
        } else if ft.is_symlink() {
            FileType::Link
        } else {
            FileType::File
        };

        Ok(ft)
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

#[derive(Debug, Clone)]
pub enum FileType {
    File,
    Dir,
    Link,
}
