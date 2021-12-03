use std::collections::VecDeque;
use std::env;
use std::path::{Path, PathBuf};

use clap::Parser;

use shelflib::graph::PackageGraph;
use shelflib::load::{LoadError, SpecLoader};

#[derive(Parser, Debug)]
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

#[inline]
pub fn cli(opts: Options) -> Result<(), ()> {
    run(opts).map_err(|_| tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}"))
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    if opts.packages.is_empty() {
        tl_info!("No packages specified, exiting");
        return Ok(());
    }

    tl_info!("Loading packages");
    let _graph = load(&opts.packages);

    Ok(())
}

#[inline]
fn load(paths: &[String]) -> Result<PackageGraph, ()> {
    let mut paths: VecDeque<_> = paths
        .iter()
        .map(|path| (PathBuf::from(path), None))
        .collect();

    let mut pg = PackageGraph::new();
    let mut errors = Vec::new();
    while let Some((path, parent)) = paths.pop_front() {
        tl_info!("Loading package '{[green]}'", path.display());
        match load_one(&path, parent, &mut pg) {
            Err(err) => {
                errors.push(err);
            }
            Ok(deps) => {
                let deps = deps.into_iter().map(|dpath| (dpath, Some(path.clone())));
                paths.extend(deps);
            }
        };
    }

    if !errors.is_empty() {
        sl_error!("{$red}Encountered errors while trying to load packages:{/$}\n");
        for err in errors {
            match err {
                LoadError::Read(err) => {
                    sl_error!("{$red}Couldn't read the package config:{/$} {}", err);
                }
                LoadError::Lua(err) => {
                    sl_error!("{$red}Couldn't evaluate Lua:{/$} {}", err);
                }
            }
        }

        Err(())
    } else {
        Ok(pg)
    }
}

#[inline]
fn load_one<P, Q>(
    path: P,
    parent: Option<Q>,
    graph: &mut PackageGraph,
) -> Result<Vec<PathBuf>, LoadError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if !graph.contains(&path) {
        let loader = SpecLoader::new(&path)?;

        sl_debug!("Reading package");
        let loader = loader.read()?;

        sl_debug!("Evaluating Lua");
        let loader = loader.eval()?;
        let data = loader.finish()?;

        let deps = data
            .dep_paths()
            .inspect(|dpath| sl_debug!("Queueing dependency '{[green]}'", dpath.display()))
            .collect();

        // Add to package graph.
        let _ = graph.add_package(&path, data);
        if let Some(parent) = parent {
            let _ = graph.add_dependency(path, parent);
        }

        sl_debug!("Finished!");

        Ok(deps)
    } else {
        Ok(vec![])
    }
}
