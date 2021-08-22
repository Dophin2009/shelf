use std::env;
use std::path::PathBuf;

use clap::Clap;
use directories_next::BaseDirs;

use crate::action::{Resolve, ResolveOpts};
use crate::cache::{Cache, DummyCache, FsCache};
use crate::error::EmptyError;
use crate::link;
use crate::load;
use crate::pathutil::PathWrapper;

const NAME: &str = "tidy";

#[derive(Clap, Debug)]
#[clap(version = clap::crate_version!(), author = clap::crate_authors!(), about = clap::crate_description!())]
pub struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    pub verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    pub quiet: bool,
    #[clap(short, long, about = "Pretend to process")]
    pub noop: bool,

    #[clap(short, long, about = "Linking destination (defaults to $HOME)")]
    pub home: Option<String>,

    #[clap(long, about = "Use an alternate cache location")]
    pub cache: Option<String>,
    #[clap(long, about = "Don't cache actions")]
    pub no_cache: bool,
    #[clap(long, about = "Clear any existing cache")]
    pub clear_cache: bool,

    pub packages: Vec<String>,
}

pub fn cli(opts: Options) -> Result<(), EmptyError> {
    match run(opts) {
        Ok(_) => Ok(()),
        Err(err) => {
            tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}");
            Err(err)
        }
    }
}

fn run(opts: Options) -> Result<(), EmptyError> {
    // FIXME error printing
    let (dest_path, cache_path) = resolve_paths(opts.home)?;

    tl_info!("Loading packages...");
    let graph = load::load_multi(&opts.packages)?;

    tl_info!("Sorting packages...");
    let packages = link::link(&dest_path, &graph)?;

    tl_info!("Starting package linking...");
    let resolve_opts = ResolveOpts {};

    let mut cache: Box<dyn Cache> = if !opts.no_cache {
        let mut cache = FsCache::empty(cache_path.abs());
        if opts.clear_cache {
            cache.clear();
        }
        Box::new(cache)
    } else {
        Box::new(DummyCache::new())
    };

    for actions in packages {
        tl_info!("Linking {$blue}{}{/$}...", actions.name());
        for action in actions {
            // FIXME support for choosing fail-fast/skip/etc. on error
            fail!(action.resolve(&resolve_opts, &mut cache));
        }
    }

    Ok(())
}

#[inline]
fn resolve_paths<'a>(dest_opt: Option<String>) -> Result<(PathWrapper, PathWrapper), EmptyError> {
    let base_dirs = failopt!(BaseDirs::new(), {
        tl_error!("Couldn't determine HOME directory")
    });

    let dest_path = match dest_opt {
        Some(p) => PathBuf::from(p),
        None => base_dirs.home_dir().to_path_buf(),
    };
    let dest = PathWrapper::from_cwd(dest_path);
    let cache = PathWrapper::from_cwd(base_dirs.cache_dir().to_path_buf());

    Ok((dest, cache))
}

#[inline]
fn home() -> Option<PathBuf> {
    home::home_dir()
}
