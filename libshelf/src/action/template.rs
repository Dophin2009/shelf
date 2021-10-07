pub struct HandlebarsAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub vars: Tree,

    pub optional: bool,
    pub partials: HandlebarsPartials,
}

impl Resolve for HandlebarsAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            src,
            dest,
            vars,
            optional,
            partials,
        } = self;

        // If file does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        if !src.exists() {
            if optional {
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skip);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // Render contents.
        let contents = fail!(templating::hbs::render(src.abs(), &vars, &partials), err => {
            sl_error!("{$red}Couldn't render Handlebars template:{/$} {}", err);
        });
        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct LiquidAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub vars: Tree,

    pub optional: bool,
}

impl Resolve for LiquidAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            src,
            dest,
            vars,
            optional,
        } = self;

        // If file does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        if !src.exists() {
            if optional {
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skip);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // Render resulting file contents.
        let contents = fail!(templating::liquid::render(src.abs(), &vars), err => {
            sl_error!("{$red}Couldn't render Liquid template:{/$} {}", err);
        });
        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}
