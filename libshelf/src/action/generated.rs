pub struct YamlAction {
    pub dest: PathWrapper,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolve for YamlAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(serde_yaml::to_string(&values), err => {
            sl_error!("{$red}Couldn't convert value map into yaml:{/$} {}", err);
        });
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct TomlAction {
    pub dest: PathWrapper,
    pub values: Tree,

    pub header: Option<String>,
}

impl Resolve for TomlAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            dest,
            values,
            header,
        } = self;

        let mut contents = fail!(toml::to_string_pretty(&values), err => {
            sl_error!("{$red}Couldn't convert value map into toml:{/$} {}", err);
        });
        contents = match header {
            Some(header) => format!("{}\n{}", header, contents),
            None => contents,
        };

        let wa = WriteAction { dest, contents };
        wa.resolve(opts, cache)
    }
}

pub struct JsonAction {
    pub dest: PathWrapper,
    pub values: Tree,
}

impl Resolve for JsonAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self { dest, values } = self;

        let contents = fail!(serde_json::to_string(&values), err => {
            sl_error!("{$red}Couldn't convert value map into json:{/$} {}", err);
        });

        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        wa.resolve(opts, cache)
    }
}
