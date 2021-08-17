use std::collections::HashMap;
use std::fmt;

use mlua::Lua;
use petgraph::{algo, graphmap::DiGraphMap};

use crate::pathutil::PathWrapper;
use crate::spec::Spec;

pub struct PackageState {
    /// Absolute path of the package.
    pub path: PathWrapper,
    /// Package specification.
    pub data: Spec,
    /// Saved Lua state.
    pub lua: Lua,
}

pub struct PackageGraph {
    /// Directional graph of package dependencies.
    pub(crate) graph: DiGraphMap<u64, ()>,
    /// Map storing the path and package data.
    pub(crate) map: HashMap<u64, PackageState>,
}

impl PackageGraph {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::new(),
            map: HashMap::new(),
        }
    }

    #[inline]
    pub fn graph(&self) -> &DiGraphMap<u64, ()> {
        &self.graph
    }

    #[inline]
    pub fn data(&self) -> &HashMap<u64, PackageState> {
        &self.map
    }

    #[inline]
    pub fn order<'g>(&'g self) -> Result<Vec<&'g PackageState>, CircularDependencyError> {
        let mut sorted = match algo::toposort(&self.graph, None) {
            Ok(v) => v,
            Err(cycle) => {
                let node_id = cycle.node_id();
                let ps = self.map.get(&node_id).unwrap();
                return Err(CircularDependencyError(ps.path.clone()).into());
            }
        };
        sorted.reverse();

        let v: Vec<_> = sorted
            .into_iter()
            .map(|id| self.map.get(&id).unwrap())
            .collect();
        Ok(v)
    }
}

#[derive(Debug, Clone)]
pub struct CircularDependencyError(pub PathWrapper);

impl fmt::Display for CircularDependencyError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Circular dependency found for package: {}",
            self.0.absd()
        )
    }
}

impl std::error::Error for CircularDependencyError {}
