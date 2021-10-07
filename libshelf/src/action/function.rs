pub struct FunctionAction<'lua> {
    pub function: Function<'lua>,

    pub start: PathWrapper,
    pub error_exit: NonZeroExitBehavior,
}

impl<'a> Resolve for FunctionAction<'a> {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            function,
            start,
            error_exit,
        } = self;

        let cwd = env::current_dir().unwrap();
        env::set_current_dir(start.abs()).unwrap();

        sl_debug!("Calling function...");
        let ret: mlua::Value = fail!(function.call(()), err => {
            sl_error!("{$red}Couldn't finish executing function hook:{/$} {}", err);
        });

        match ret {
            mlua::Value::Nil => {}
            v => match error_exit {
                NonZeroExitBehavior::Error => {
                    sl_error!("{$red}Function returned with an error:{/$} {:?}", v);
                    return Err(EmptyError);
                }
                NonZeroExitBehavior::Warn => {
                    sl_warn!("Done... {$yellow}non-nil exit:{/$} {:?}", v)
                }
                NonZeroExitBehavior::Ignore => {
                    sl_debug!("Done... exit {$blue}nil{/$}");
                }
            },
        }

        // FIXME restore cwd regardless of error or not
        env::set_current_dir(&cwd).unwrap();
        Ok(Resolution::Done)
    }
}
