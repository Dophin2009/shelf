use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use mlua::Lua;
use petgraph::{algo, graphmap::DiGraphMap};

use crate::spec::Spec;

pub struct PackageState {
    /// Absolute path of the package.
    pub path: PathBuf,
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
    pub fn order<'g>(&'g self) -> Result<impl Iterator<Item = &'g PackageState>> {
        let mut sorted = match algo::toposort(&self.graph, None) {
            Ok(v) => v,
            Err(cycle) => {
                return Err(anyhow!(
                    "Circular dependency encountered: {}",
                    cycle.node_id()
                ))
            }
        };
        sorted.reverse();

        let iter: Vec<_> = sorted
            .into_iter()
            .map(|id| -> Result<_> {
                let tup = self
                    .map
                    .get(&id)
                    .ok_or_else(|| anyhow!("Package identifier not found: {}", id))?;
                Ok(tup)
            })
            .collect::<Result<_>>()?;
        Ok(iter.into_iter())
    }
}
