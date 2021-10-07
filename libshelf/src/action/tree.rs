pub struct TreeAction {
    pub src: PathWrapper,
    pub dest: PathWrapper,
    pub globs: Patterns,
    pub ignore: Patterns,

    pub copy: bool,
    pub optional: bool,
}

impl Resolve for TreeAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            src,
            dest,
            globs,
            ignore,
            copy,
            optional,
        } = self;

        // If src does not exist, and optional flag enabled, skip.
        // If optional flag disabled, error.
        // If src exists but isn't a directory, and optional flag enabled, skip it.
        // If optional flag disabled, return error.
        if !src.exists() || !src.is_dir() {
            if optional {
                // log_skip(format!("{[green]} does not exist", src.reld()));
                return Ok(Resolution::Skip);
            } else {
                log_miss(&src);
                return Err(EmptyError);
            }
        }

        // FIXME handle absolute path globs
        #[inline]
        fn glob_tree(
            src: &PathWrapper,
            pats: &Vec<String>,
        ) -> Result<HashSet<PathBuf>, EmptyError> {
            let cwd = env::current_dir().unwrap();
            env::set_current_dir(src.abs()).unwrap();

            let matches: Vec<glob::Paths> = pats
                .iter()
                .map(|pat| glob::glob(pat))
                .map(|r| {
                    r.map_err(|err| {
                        // FIXME path in error
                        sl_error!("{$red}Couldn't glob a pattern:{/$} {}", err);
                        EmptyError
                    })
                })
                .collect::<Result<_, _>>()?;

            let res = matches
                .into_iter()
                .flatten()
                .filter_map(|r| match r {
                    Ok(path) if path.is_file() => Some(Ok(path)),
                    Ok(_) => None,
                    Err(err) => {
                        // FIXME path in error
                        sl_error!("{$red}Couldn't read path while globbing:{/$} {}", err);
                        Some(Err(EmptyError))
                    }
                })
                .collect::<Result<_, _>>()?;

            env::set_current_dir(&cwd).unwrap();

            Ok(res)
        }

        // Glob to get file paths.
        let mut paths = glob_tree(&src, &globs)?;
        // Glob to get ignored paths.
        let ignore_paths = glob_tree(&src, &ignore)?;

        // Remove all the ignored paths from the globbed paths.
        for path in ignore_paths {
            paths.remove(&path);
        }

        let src_paths = paths.iter().map(|path| src.join(path));
        let dest_paths = paths.iter().map(|path| dest.join(path));

        // Map paths and dest paths into linking actions.
        let it = src_paths.zip(dest_paths).map(move |(fsrc, fdest)| {
            Action::Link(LinkAction {
                src: fsrc,
                dest: fdest,
                copy,
                optional: false,
            })
        });

        // FIXME handle resolutions
        it.map(|action| action.resolve(opts, cache))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Resolution::Done)
    }
}
