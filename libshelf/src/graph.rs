use std::collections::{
    hash_map::{self, DefaultHasher},
    HashMap,
};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use mlua::Lua;
use petgraph::graphmap::Nodes;
use petgraph::{algo, graphmap::DiGraphMap};

use crate::spec::Spec;

pub struct PackageData {
    /// Absolute path of the package.
    pub path: PathBuf,
    /// Package specification.
    pub spec: Spec,
    /// Saved Lua state.
    pub lua: Lua,
}

pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<u64, ()>,
    /// Map storing package data.
    datamap: HashMap<u64, PackageData>,
}

impl PackageGraph {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::<u64, ()>::new(),
            datamap: HashMap::new(),
        }
    }

    #[inline]
    pub fn get<P>(&self, path: P) -> Option<&PackageData>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(path);
        self.datamap.get(&id)
    }

    #[inline]
    pub fn contains<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(path);
        self.datamap.contains_key(&id)
    }

    /// Inserts a package to the graph, returning the existing data if it exists and `None` if it
    /// does not.
    #[inline]
    pub fn add_package<P>(&mut self, path: P, data: PackageData) -> Option<PackageData>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(path);

        let existing = self.datamap.insert(id, data);
        if existing.is_none() {
            self.graph.add_node(id);
        }

        existing
    }

    /// Removes a package from the graph, returning the data if it exists and `None` if it does
    /// not.
    #[inline]
    pub fn remove_package<P>(&mut self, path: P) -> Option<PackageData>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(path);

        let data = self.datamap.remove(&id)?;
        self.graph.remove_node(id);

        Some(data)
    }

    /// Returns true if the graph contains the package.
    #[inline]
    pub fn contains_package<P>(&mut self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(path);
        self.datamap.contains_key(&id)
    }

    /// Adds a dependency relation between two packages, returning true if it is successfully added
    /// (both packages exist in the graph and the relation does not already exist) and false
    /// otherwise.
    #[inline]
    pub fn add_dependency<P, Q>(&mut self, path: P, parent: Q) -> bool
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let id = self.keyid(&path);
        let pid = self.keyid(&path);
        match (self.datamap.get(&id), self.datamap.get(&pid)) {
            (Some(_), Some(_)) => {
                self.graph.add_edge(pid, id, ());
                true
            }
            _ => false,
        }
    }

    /// Removes a dependency relation between two packages, returning true if it is successfully
    /// removed and false if it does not exist.
    #[inline]
    pub fn remove_dependency<P, Q>(&mut self, path: P, parent: Q) -> bool
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let id = self.keyid(&path);
        let pid = self.keyid(&path);
        match (self.datamap.get(&id), self.datamap.get(&pid)) {
            (Some(_), Some(_)) => self.graph.remove_edge(pid, id).is_some(),
            _ => false,
        }
    }

    /// Check if a dependency relation between two packages exists.
    #[inline]
    pub fn contains_dependency<P, Q>(&mut self, path: P, parent: Q) -> bool
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let id = self.keyid(&path);
        let pid = self.keyid(&path);

        self.datamap.contains_key(&id)
            && self.datamap.contains_key(&pid)
            && self.graph.contains_edge(pid, id)
    }

    /// Clears all packages and dependency relations from the graph.
    #[inline]
    pub fn clear(&mut self) {
        self.datamap.clear();
        self.graph.clear();
    }

    /// Returns the number of packages in the graph.
    #[inline]
    pub fn package_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the number of dependency relations in the graph.
    #[inline]
    pub fn dependency_count(&self) -> usize {
        self.graph.edge_count()
    }

    #[inline]
    pub fn iter(&self) -> PackageIter<'_> {
        PackageIter {
            inner: self.datamap.iter(),
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> PackageIterMut<'_> {
        PackageIterMut {
            inner: self.datamap.iter_mut(),
        }
    }

    /// Returns an iterator of packages in topological sort order, with dependencies coming before
    /// dependents.
    #[inline]
    pub fn order<'g>(&'g self) -> Result<Vec<&'g PackageData>, CircularDependencyError> {
        let mut sorted = match algo::toposort(&self.graph, None) {
            Ok(v) => v,
            Err(cycle) => {
                let node_id = cycle.node_id();
                let ps = self.datamap.get(&node_id).unwrap();
                return Err(CircularDependencyError(ps.path.clone()).into());
            }
        };
        sorted.reverse();

        let vec = sorted
            .into_iter()
            .map(|id| {
                // This is safe because data is guaranteed to be in map.
                self.datamap.get(&id).unwrap()
            })
            .collect();
        Ok(vec)
    }

    #[inline]
    fn keyid<P: AsRef<Path>>(&self, path: P) -> u64 {
        let path = path.as_ref().to_path_buf();
        let mut s = DefaultHasher::new();
        path.hash(&mut s);
        s.finish()
    }
}

pub struct PackageIter<'g> {
    inner: hash_map::Iter<'g, u64, PackageData>,
}

impl<'g> Iterator for PackageIter<'g> {
    type Item = &'g PackageData;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

pub struct PackageIterMut<'g> {
    inner: hash_map::IterMut<'g, u64, PackageData>,
}

impl<'g> Iterator for PackageIterMut<'g> {
    type Item = &'g mut PackageData;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

#[derive(Debug, Clone)]
pub struct CircularDependencyError(pub PathBuf);

impl fmt::Display for CircularDependencyError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "detected circular dependency for package: {}",
            self.0.display()
        )
    }
}

impl std::error::Error for CircularDependencyError {}