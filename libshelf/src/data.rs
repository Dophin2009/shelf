use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;

use mlua::Lua;
use petgraph::{algo, graphmap::DiGraphMap};

use crate::pathutil::PathWrapper;
use crate::spec::{Directive, Spec};

pub struct PackageData {
    /// Absolute path of the package.
    pub path: PathWrapper,
    /// Package specification.
    pub spec: Spec,
    /// Saved Lua state.
    pub lua: Lua,
}

impl PackageData {
    #[inline]
    pub fn directive_data<'p>(&'p self) -> impl Iterator<Item = DirectiveData<'p>> + 'p {
        self.spec.directives.iter().map(|d| DirectiveData {
            path: &self.path,
            directive: d,
            lua: &self.lua,
        })
    }
}

pub struct DirectiveData<'p> {
    pub path: &'p PathWrapper,
    pub directive: &'p Directive,
    pub lua: &'p Lua,
}

type PackageId = u64;
pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<PackageId, ()>,
    /// Map storing the path and package data.
    map: HashMap<PackageId, PackageData>,
}

impl PackageGraph {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::<PackageId, ()>::new(),
            map: HashMap::new(),
        }
    }

    #[inline]
    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<&PackageData> {
        let id = self.hash_key(path);
        self.map.get(&id)
    }

    #[inline]
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        let id = self.hash_key(path);
        self.map.contains_key(&id)
    }

    #[inline]
    pub fn add<P: AsRef<Path>>(&mut self, path: P, data: PackageData) -> bool {
        let id = self.hash_key(path);
        match self.map.get(&id) {
            Some(_) => false,
            None => {
                self.map.insert(id, data);
                self.graph.add_node(id);
                true
            }
        }
    }

    #[inline]
    pub fn add_parent<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, path: P, parent: Q) -> bool {
        let id = self.hash_key(path);
        let pid = self.hash_key(parent);
        match (self.map.get_mut(&id), self.map.get(&pid)) {
            (Some(_), Some(_)) => {
                self.graph.add_edge(pid, id, ());
                true
            }
            _ => false,
        }
    }

    #[inline]
    pub fn order<'g>(&'g self) -> Result<Vec<&'g PackageData>, CircularDependencyError> {
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

    #[inline]
    fn hash_key<P: AsRef<Path>>(&self, path: P) -> u64 {
        let path = path.as_ref().to_path_buf();
        let mut s = DefaultHasher::new();
        path.hash(&mut s);
        s.finish()
    }
}

#[derive(Debug, Clone)]
pub struct CircularDependencyError(pub PathWrapper);

impl fmt::Display for CircularDependencyError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "detected circular dependency for package: {}",
            self.0.absd()
        )
    }
}

impl std::error::Error for CircularDependencyError {}
