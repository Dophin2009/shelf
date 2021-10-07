use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;

use clap::Clap;
use directories_next::BaseDirs;

// use libshelf::action::{Resolve, ResolveOpts};
// use libshelf::cache::{Cache, DummyCache, FsCache};
use libshelf::graph::PackageGraph;
// use libshelf::link::{self, PackageIter};
use libshelf::load::SpecLoader;
use libshelf::pathutil::PathWrapper;
use libshelf::spec::Dep;

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
    // let it = link(&opts.dest, &graph)?;

    // let mut cache = get_cache(opts.no_cache, cache_path);

    // let resolve_opts = ResolveOpts {};
    // execute(it, resolve_opts, &mut cache)?;

    Ok(())
}

// fn get_cache<C: Cache>(no_cache: bool, path: PathWrapper) -> C {
// let mut cache: Box<dyn Cache> = if !opts.no_cache {
// let mut cache = FsCache::empty(cache_path.abs());
// if opts.clear_cache {
// cache.clear();
// }
// cache
// } else {
// DummyCache::new()
// };
// }

fn load_one(
    path: PathWrapper,
    parent: PathBuf,
    pg: &mut PackageGraph,
) -> Result<Vec<PathBuf>, LoadError> {
    if !pg.contains(path.abs()) {
        tl_info!("Loading package {[green]}", path.reld());
        let loader = SpecLoader::new(path.clone())?;

        sl_debug!("Reading package...");
        let loader = loader.read()?;

        sl_debug!("Evaluating Lua...");
        let loader = loader.eval()?;
        let data = loader.to_package_data()?;

        let deps = data
            .spec
            .deps
            .iter()
            .map(|Dep { path }| path.clone())
            .collect();

        // Add to package graph.
        let _ = pg.add_package(path.abs(), data);
        let _ = pg.add_parent(path.abs(), parent);

        Ok(Some((path.abs().to_path_buf(), deps)))
    } else {
        Ok(None)
    }
}

fn load(paths: &[String]) -> Result<PackageGraph, ()> {
    tl_info!("Loading packages...");

    let paths: VecDeque<_> = paths
        .iter()
        .map(|path| PathWrapper::from_cwd(path))
        .map(|path| (path, None))
        .collect();

    let pg = PackageGraph::new();
    let errors = Vec::new();
    while let Some((path, parent)) = paths.pop_front() {
        let deps = match load_one(path, parent, &mut pg) {
            Ok(v) => v,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };

        let dep_iter = deps.into_iter().map(|dep| (path.clone(), Some(dep)));
        paths.extend(dep_iter);
    }

    if !errors.is_empty() {
        sl_error!("{$red}Encountered errors while trying to load packages:{/$}\n");
        for error in errors {
            match err {
                LoadError::Read(err) => {
                    sl_error!("{$red}Couldn't read the package config:{/$}", err);
                    sl_i_error!("Location: {[green]}", path);
                }
                LoadError::Lua(err) => {
                    sl_error!("{$red}Couldn't evaluate Lua:{/$}", err);
                }
            }
        }

        Err(())
    } else {
        Ok(pg)
    }
}

// fn link<'d, 'p, P: AsRef<Path>>(
// dest: P,
// graph: &PackageGraph,
// ) -> Result<impl Iterator<Item = PackageIter<'d, 'p>>, ()> {
// tl_info!("Starting package linking...");
// match link::link(&dest_path, &graph) {
// Ok(it) => it,
// Err(err) => {
// sl_error!(
// "{$red}Circular dependency found for package:{/$} {}",
// err.0.absd()
// );
// Err(())
// }
// }
// }

// fn execute<'d, 'p>(
// it: impl Iterator<Item = PackageIter<'d, 'p>>,
// opts: ResolveOpts,
// cache: &mut dyn Cache,
// ) -> Result<(), ()> {
// for package_actions in it {
// tl_info!("Linking {$blue}{}{/$}...", actions.name());
// for action in package_actions {
// // FIXME support for choosing fail-fast/skip/etc. on error
// action.resolve(&resolve_opts, &mut cache);
// }
// }

// Ok(())
// }

fn resolve_paths<'a>(dest_opt: Option<String>) -> Result<(PathWrapper, PathWrapper), ()> {
    let base_dirs = match BaseDirs::new() {
        Some(bd) => bd,
        None => {
            tl_error!("Couldn't determine HOME directory");
            return Err(());
        }
    };

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
