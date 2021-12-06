pub mod output;

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
        output::error_loading();
        for (path, err) in errors.into_iter() {
            output::error_loading_path(&path, err);
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
    let deps = if graph.contains(path.abs()) {
        output::info_loading_skip(path);

        vec![]
    } else {
        output::info_loading(path);

        let loader = SpecLoader::new(&path.abs())?;

        output::debug_reading();
        let loader = loader.read()?;

        output::debug_evaling();
        let loader = loader.eval()?;
        let data = loader.finish()?;

        let deps = data
            .dep_paths()
            .map(|dpath| CtxPath::from_cwd(dpath))
            .inspect(|dpath| output::debug_queue_dep(dpath, &data.path))
            .collect();

        // Add to package graph.
        let _ = graph.add_package(data);

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
