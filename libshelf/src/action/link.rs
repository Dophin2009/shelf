pub struct LinkAction {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub copy: bool,
    pub optional: bool,
}

impl Resolve for LinkAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
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

        // FIXME cache this action
        if copy {
            self.resolve_copy(opts, cache)
        } else {
            self.resolve_link(opts, cache)
        }
    }
}

impl LinkAction {
    // FIXME implement missing pieces
    #[inline]
    fn resolve_copy<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        // Check the cache for the destination path.
        let dest_cache = cache.get(self.dest);
        match (dest.exists(), dest_cache) {
            // Destination file doesn't exist, but is found in cache.
            // Warn about changes that might've occurred since last run.
            (false, Some(dest_cache)) => {
                notices.push(Notice::Warn(WarnNotice::ManualChange {
                    path: dest.clone(),
                }));

                // sl_warn!("{$yellow}Destination was found in cache, but no file exists there.{/$}");
                // sl_warn!("{$yellow}Manual changes may have removed this file since last time.{/$}");
                // sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
            }
            // Destination file exists, but is not found in cache.
            (true, None) => {
                // Emit warning about overwriting.
                notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                // Add op to remove the existing.
                ops.push(Op::Rm(RmOp { path: dest.clone() }));

                // sl_warn!(
                // "{$yellow}Destination wasn't found in the cache, but a file exists there.{/$}"
                // );
                // sl_warn!("{$yellow}It will be replaced.{/$}");
                // sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
            }
            // Destination file exists and is found in cache.
            // Check the kind of file that should exist there.
            (true, Some(dest_cache)) => match dest_cache.typ {
                // Existing should be a normal file, check the content hashes.
                FileTyp::File { content_hash } => {
                    // FIXME: implement

                    // If the content hash doesn't match, emit a notice.
                    // If it does, do nothing.
                    unimplemented!()
                }
                // Existing should be a directory, warn about overwrite.
                FileTyp::Dir => {
                    // FIXME: implement
                    unimplemented!()
                }
                // Existing should be a symlink, check where it points.
                FileTyp::Link => {
                    // FIXME: implement
                    unimplemented!()
                }
            },

            // Destination file doesn't exist, and is not found in cache.
            // Carry on with copying.
            (false, None) => {}
        };

        // sl_debug!(
        // "Copying file: {[green]} to {[green]}",
        // src.reld(),
        // dest.reld()
        // );

        // Check for existence of parent directories.
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                ops.push(Op::Mkdir(MkdirOp {
                    path: parent.to_path_buf(),
                    parents: true,
                }))
            }
        }

        ops.push(Op::Copy(CopyOp { src, dest }));
        Ok(Resolution::Done(DoneOutput { ops, notices }))

        // fail!(res, err => {
        // sl_error!("{$red}Couldn't copy file:{/$} {}", err);
        // sl_i_error!("{$red}Source:{/$} {[green]}", src.absd());
        // sl_i_error!("{$red}Destination:{/$} {[green]}", dest.absd());
        // });
    }

    #[inline]
    fn resolve_link<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self { src, dest, .. } = self;

        let mut ops = Vec::new();
        let mut notices = Vec::new();

        let mut do_link = true;

        // sl_debug!("Found an existing file at the destination; checking...");

        // Check the cache for the destination path.
        let dest_cache = cache.get(self.dest);
        match (dest.exists(), dest_cache) {
            // Destination file exists, and is found in cache.
            // Check the kind of file that should exist there.
            (true, Some()) => match dest_cache.typ {
                // Existing should be a normal file, emit notice about overwriting.
                FileTyp::File { content_hash } => {
                    // FIXME: implement
                    unimplemented!()
                }
                // Existing is a directory, emit notice about overwriting.
                FileTyp::Dir => {
                    // FIXME: implement
                    unimplemented!()
                }
                // Existing is a symlink, check where it points.
                FileTyp::Link => {
                    // FIXME: implement
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
                    if ft.is_symlink() {
                        // Destination file is a symlink.
                        // Read the target location and check against src.
                        let dest_target = match fs::read_link(&dest) {
                            Ok(path) => path,
                            Err(err) => {
                                return Err(ResolutionError::FileReadMetadata { path: dest, err });
                                // sl_error!("{$red}Couldn't follow symlink:{/$} {}", err);
                                // sl_i_error!("{$red}Location:{/$} {[green]}", dest.absd());
                            }
                        };

                        if dest_target == src {
                            // Destination symlink target and src are the same. Emit a notice.
                            notices.push(Notice::Info(InfoNotice::ExistingSymlink {
                                path: dest.clone(),
                                target: dest_target,
                            }));

                            // Don't link.
                            do_link = false;

                            // sl_debug!("Symlink was already established; doing nothing...");
                        } else {
                            // Destination symlink target and src are not the same. Emit a warning
                            // about overwrite.
                            notices
                                .push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                            // Add op to remove the existing symlink.
                            ops.push(Op::Rm(RmOp {
                                path: dest,
                                dir: false,
                            }));

                            // sl_warn!("{$yellow}An existing symlink (pointing to a different location) was found at the destination{/$}");
                            // sl_i_warn!("{$yellow}Destination:{/$} {[green]}", dest.absd());
                            // sl_i_warn!("{$yellow}It will be replaced.{/$}");
                        }
                    } else {
                        // Destination file is not a symlink.
                        // Emit a warning about possible manual action.
                        notices.push(Notice::Warn(WarnNotice::ManualChange {
                            path: dest.clone(),
                        }));
                        // Emit a warning about overwriting.
                        notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                        // Add op to remove the existing file.
                        let is_dir = !dest.is_file();
                        ops.push(Op::Rm(RmOp {
                            path: dest,
                            dir: is_dir,
                        }));

                        // sl_warn!("{$yellow}An existing file or directory was found at the destination{/$}");
                        // sl_i_warn!("{$yellow}Location:{/$} {[green]}", dest.absd());
                        // sl_i_warn!("{$yellow}It will be replaced.{/$}");
                    }
                }
            },
            // Destination file exists, but is not found in cache.
            // Emit warning about overwrite.
            (true, None) => {
                notices.push(Notice::Warn(WarnNotice::Overwrite { path: dest.clone() }));

                // FIXME: Check existing file as with above case.
                unimplemented!()
            }
            // Destination file doesn't exist, and is found in cache.
            // Emit warning about possible manual change.
            (false, Some()) => {
                notices.push(Notice::Warn(WarnNotice::ManualChange {
                    path: dest.clone(),
                }));
            }
            // Destination file doesn't exist, and is not found in cache.
            // Carry on with linking.
            (false, None) => {}
        }

        if do_link {
            ops.push(Op::Link(LinkOp { src, dest }));
        }

        // sl_debug!(
        // "Linking file: {[green]} to {[green]}",
        // src.reld(),
        // dest.reld()
        // );

        Ok(Resolution::Done(DoneOutput { ops, notices }))
    }
}
