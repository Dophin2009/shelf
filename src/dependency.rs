use crate::config::Package;

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use petgraph::algo;
use petgraph::graphmap::DiGraphMap;

#[derive(Debug)]
pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<u64, ()>,
    /// Map storing the path and package
    map: HashMap<u64, (PathBuf, Package)>,
}

impl PackageGraph {
    /// Build a package dependency graph from a package. `path` must be the absolute path of the
    /// package.
    pub fn from_package(path: PathBuf, root: Package) -> Result<Self> {
        let mut new = PackageGraph {
            graph: DiGraphMap::new(),
            map: HashMap::new(),
        };

        new.add_package(path, root)?;

        Ok(new)
    }

    /// Add a package to the graph and map by absolute path.
    fn add_package(&mut self, path: PathBuf, package: Package) -> Result<()> {
        let dependencies = package.config.dependencies.clone();

        let id = hash_path(&path);
        self.graph.add_node(id);
        self.map.insert(id, (path.clone(), package));

        // Add dependencies of the package.
        for dep_path_rel in &dependencies {
            let dep_path_abs = path.join(dep_path_rel);
            let dep_path = fs::canonicalize(dep_path_abs)?;
            let dep = Package::from_directory(&dep_path)?;

            let dep_id = hash_path(&dep_path);
            self.graph.add_edge(id, dep_id, ());

            self.add_package(dep_path, dep)?;
        }

        Ok(())
    }

    pub fn topological_order(&self) -> Result<impl Iterator<Item = &(PathBuf, Package)>> {
        let mut sorted = match algo::toposort(&self.graph, None) {
            Ok(v) => v,
            Err(cycle) => {
                return Err(anyhow!(
                    "Circular dependency encountered: {}",
                    cycle.node_id()
                ))?
            }
        };
        sorted.reverse();

        let iter: Vec<_> = sorted
            .into_iter()
            .map(|id| -> Result<_> {
                let tup = self
                    .map
                    .get(&id)
                    .ok_or(anyhow!("Package identifier not found: {}", id))?;
                Ok(tup)
            })
            .collect::<Result<_>>()?;
        Ok(iter.into_iter())
    }
}

fn hash_path(path: &PathBuf) -> u64 {
    let mut s = DefaultHasher::new();
    path.hash(&mut s);
    s.finish()
}

// #[derive(Debug)]
// pub struct CircularDependencyError(Cycle<&'a str>);

// impl<'a> error::Error for CircularDependencyError<'a> {}

// impl<'a> fmt::Display for CircularDependencyError<'a> {
// fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
// write!(f, "Circular dependency encountered")
// }
// }
