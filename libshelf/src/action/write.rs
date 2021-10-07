pub struct WriteAction {
    pub dest: PathWrapper,
    pub contents: String,
}

impl Resolve for WriteAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self { dest, contents } = self;

        // If the destination doesn't exist yet, create the directories and write the file.
        if !dest.exists() {
            mkdir_parents(&dest)?;

            sl_debug!("Writing file: {[green]}", dest.reld());
            fail!(fs::write(&dest.abs(), &contents), err => {
                sl_error!("{$red}Couldn't write{/$} {[green]} {$red}:{/$} {}", dest.absd(), err);
            });

            sl_info!("Done... {$green}ok!{/$}");

            // FIXME cache this action
            Ok(Resolution::Done)
        } else {
            todo!();

            // Retrieve information for this location from the cache.
            // If not found, we should error.

            // If existing is a symlink, replace it.

            // If existing is not a symlink, replace it.

            // Cache the action.

            Ok(Resolution::Done)
        }
    }
}
