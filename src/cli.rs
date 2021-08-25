use std::env;
use std::path::PathBuf;

use clap::Clap;
use directories_next::BaseDirs;

use libshelf::action::{Resolve, ResolveOpts};
use libshelf::cache::{Cache, DummyCache, FsCache};
use libshelf::link;
use libshelf::load::{Loader, LoaderError};
use libshelf::pathutil::PathWrapper;

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

pub fn cli(opts: Options) -> Result<(), ()> {
    match run(opts) {
        Ok(_) => Ok(()),
        Err(err) => {
            tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}");
            Err(())
        }
    }
}

macro_rules! fail {
    ($res:expr) => {
        fail!($res, _err => {})
    };
    ($res:expr, $err:ident => $block:block) => {
        match $res {
            Ok(v) => v,
            Err($err) => {
                $block;
                return Err(());
            }
        }
    };
}

macro_rules! failopt {
    ($res:expr) => {
        failopt!($res, {})
    };
    ($res:expr, $block:block) => {
        match $res {
            Some(v) => v,
            None => {
                $block;
                return Err(());
            }
        }
    };
}

fn run(opts: Options) -> Result<(), ()> {
    // FIXME error printing
    let (dest_path, cache_path) = resolve_paths(opts.home)?;

    let graph = load(&opts.packages)?;
    let packages = link(&opts.dest, &graph)?;

    let mut cache: Box<dyn Cache> = if !opts.no_cache {
        let mut cache = FsCache::empty(cache_path.abs());
        if opts.clear_cache {
            cache.clear();
        }
        Box::new(cache)
    } else {
        Box::new(DummyCache::new())
    };

    let resolve_opts = ResolveOpts {};
    for actions in packages {
        tl_info!("Linking {$blue}{}{/$}...", actions.name());
        for action in actions {
            // FIXME support for choosing fail-fast/skip/etc. on error
            action.resolve(&resolve_opts, &mut cache);
        }
    }

    Ok(())
}

fn load(paths: &[String]) -> Result<PackageGraph, ()> {
    tl_info!("Loading packages...");

    let mut loader = Loader::new();
    paths.iter().for_each(|path| loader.add(path));

    match loader.load(&opts.packages) {
        Ok(graph) => graph,
        Err(err) => {
            sl_error!("{$red}Encountered errors while loading packages:{/$}\n");

            err.errors.0.iter().for_each(|(&err, path)| {
                sl_error!("In {[green]}:", path.absd());
                match err {
                    LoadError::Read(err) => {
                        sl_i_error!("{$red}Couldn't read the package config:{/$}", err);
                    }
                    LoadError::Lua(err) => {
                        sl_i_error!("{$red}Couldn't evaluate Lua:{/$}", err);
                    }
                }
            });
            Err(())
        }
    }
}

fn link<'d, 'p, P: AsRef<Path>>(
    dest: P,
    graph: &PackageGraph,
) -> Result<impl Iterator<Item = PackageIter<'d, 'p>>, ()> {
    tl_info!("Starting package linking...");
    match link::link(&dest_path, &graph) {
        Ok(it) => it,
        Err(err) => {
            sl_error!(
                "{$red}Circular dependency found for package:{/$} {}",
                err.0.absd()
            );
            Err(())
        }
    }
}

fn resolve_paths<'a>(dest_opt: Option<String>) -> Result<(PathWrapper, PathWrapper), ()> {
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
