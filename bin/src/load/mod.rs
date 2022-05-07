mod output;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use shelflib::{
    graph::PackageGraph,
    load::{LoadError, SpecLoader},
};

use crate::ctxpath::CtxPath;

#[derive(Debug)]
pub struct Loaded {
    pub graph: PackageGraph,
    pub paths: HashMap<PathBuf, CtxPath>,
}

#[derive(Debug)]
pub struct Loader {
    packages: VecDeque<(CtxPath, Option<CtxPath>)>,
    graph: PackageGraph,
    paths: HashMap<PathBuf, CtxPath>,
}

impl Loader {
    pub fn new(packages: Vec<PathBuf>) -> Self {
        let packages = packages
            .into_iter()
            .map(|path| (CtxPath::from_cwd(path), None))
            .collect();
        Self {
            packages,
            graph: PackageGraph::new(),
            paths: HashMap::new(),
        }
    }

    #[inline]
    pub fn load(mut self) -> Result<Loaded, ()> {
        let mut errors = Vec::new();
        while let Some((path, parent)) = self.packages.pop_front() {
            match self.load_one(&path, parent.as_ref()) {
                Err(err) => {
                    errors.push((path, err));
                }
                Ok(deps) => {
                    let deps = deps.into_iter().map(|dpath| (dpath, Some(path.clone())));
                    self.packages.extend(deps);
                    self.paths.insert(path.abs().to_path_buf(), path);
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
            Ok(Loaded {
                graph: self.graph,
                paths: self.paths,
            })
        }
    }

    #[inline]
    fn load_one(
        &mut self,
        path: &CtxPath,
        parent: Option<&CtxPath>,
    ) -> Result<Vec<CtxPath>, LoadError> {
        output::info_loading(path);
        let deps = if self.graph.contains(path.abs()) {
            output::debug_skip(path);
            vec![]
        } else {
            let loader = SpecLoader::new(&path.abs())?;

            output::debug_reading();
            let loader = loader.read()?;

            output::debug_evaling();
            let loader = loader.eval()?;
            let data = loader.finish()?;

            let deps = data
                .dep_paths()
                .map(CtxPath::from_cwd)
                .inspect(|dpath| output::debug_queue_dep(dpath, &data.path))
                .collect();

            // Add to package graph.
            let _ = self.graph.add_package(data);
            deps
        };

        if let Some(parent) = parent {
            let success = self.graph.add_dependency(path.abs(), parent.abs());
            if !success {
                unreachable!();
            }
        }

        Ok(deps)
    }
}
