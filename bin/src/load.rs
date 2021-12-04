use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use shelflib::{
    graph::PackageGraph,
    load::{LoadError, SpecLoader},
};

use crate::ctxpath::CtxPath;

#[inline]
pub fn load(paths: Vec<PathBuf>) -> Result<(PackageGraph, HashMap<PathBuf, CtxPath>), ()> {
    let mut paths: VecDeque<_> = paths
        .into_iter()
        .map(|path| (CtxPath::from_cwd(path), None))
        .collect();

    let mut pg = PackageGraph::new();
    let mut pm = HashMap::new();
    let mut errors = Vec::new();
    while let Some((path, parent)) = paths.pop_front() {
        match load_one(&path, parent.as_ref(), &mut pg) {
            Err(err) => {
                errors.push((path, err));
            }
            Ok(deps) => {
                let deps = deps.into_iter().map(|dpath| (dpath, Some(path.clone())));
                paths.extend(deps);
                pm.insert(path.abs().to_path_buf(), path);
            }
        };
    }

    if !errors.is_empty() {
        sl_error!("{$bold}{$red}fatal:{/$} encountered errors while trying to load packages{/$}");
        for (path, err) in errors {
            sl_i_error!("in '{[green]}':", path.abs().display());
            match err {
                // TODO: More specific error messages
                LoadError::Read(err) => {
                    sl_ii_error!(
                        "couldn't read the package config: are you sure it exists ({$dimmed}package.lua{/$})?",
                    );
                }
                LoadError::Lua(err) => {
                    sl_ii_error!("couldn't evaluate Lua: {}",  err);
                }
            }
        }

        Err(())
    } else {
        Ok((pg, pm))
    }
}

#[inline]
fn load_one(
    path: &CtxPath,
    parent: Option<&CtxPath>,
    graph: &mut PackageGraph,
) -> Result<Vec<CtxPath>, LoadError> {
    tl_info!("Loading package '{[green]}'", path.rel().display());

    let deps = if graph.contains(path.abs()) {
        tl_info!("Already done, skipping!");
        vec![]
    } else {
        let loader = SpecLoader::new(&path.abs())?;

        sl_debug!("Reading package");
        let loader = loader.read()?;

        sl_debug!("Evaluating Lua");
        let loader = loader.eval()?;
        let data = loader.finish()?;

        let deps = data
            .dep_paths()
            .map(|dpath| CtxPath::from_cwd(dpath))
            .inspect(|dpath| {
                let dpath_rel = CtxPath::new(dpath.abs(), &data.path).unwrap();
                sl_debug!("Queueing dependency '{[green]}'", dpath_rel.rel().display())
            })
            .collect();

        // Add to package graph.
        let _ = graph.add_package(data);

        sl_debug!("Finished!");

        deps
    };

    if let Some(parent) = parent {
        let success = graph.add_dependency(path.abs(), parent.abs());
        if !success {
            unreachable!();
        }
    }

    Ok(deps)
}
