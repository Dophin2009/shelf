use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use shelflib::graph::PackageGraph;
use shelflib::load::{LoadError, SpecLoader};

#[inline]
pub fn load(paths: &[String]) -> Result<PackageGraph, ()> {
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
    let deps = if !graph.contains(&path) {
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
        let _ = graph.add_package(data);

        sl_debug!("Finished!");

        deps
    } else {
        vec![]
    };

    if let Some(parent) = parent {
        let success = graph.add_dependency(&path, parent);
        if !success {
            unreachable!();
        }
    }

    Ok(deps)
}
