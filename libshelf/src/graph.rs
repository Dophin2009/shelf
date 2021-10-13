use std::collections::{
    hash_map::{self, DefaultHasher},
    HashMap,
};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::vec;

use mlua::Lua;
use petgraph::{
    algo,
    graphmap::{DiGraphMap, Nodes},
};

use crate::spec::Spec;

pub struct PackageData {
    /// Absolute path of the package.
    pub path: PathBuf,
    /// Package specification.
    pub spec: Spec,
    /// Saved Lua state.
    pub lua: Lua,
}

impl fmt::Debug for PackageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageData")
            .field("path", &self.path)
            .field("spec", &self.spec)
            .field("lua", &"<lua>")
            .finish()
    }
}

#[derive(Debug)]
pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<u64, ()>,
    /// Map storing package data.
    datamap: HashMap<u64, (PathBuf, PackageData)>,
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
    pub fn get<P>(&self, path: P) -> Option<(&PathBuf, &PackageData)>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(&path);
        self.datamap.get(&id).map(|(path, data)| (path, data))
    }

    #[inline]
    pub fn contains<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(&path);
        self.datamap.contains_key(&id)
    }

    /// Inserts a package to the graph, returning the existing data if it exists and `None` if it
    /// does not.
    #[inline]
    pub fn add_package<P>(&mut self, path: P, data: PackageData) -> Option<PackageData>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(&path);

        let path = path.as_ref().to_path_buf();
        let existing = self.datamap.insert(id, (path, data));
        match existing {
            Some((_, data)) => Some(data),
            None => {
                self.graph.add_node(id);
                None
            }
        }
    }

    /// Removes a package from the graph, returning the data if it exists and `None` if it does
    /// not.
    #[inline]
    pub fn remove_package<P>(&mut self, path: P) -> Option<PackageData>
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(&path);

        let data = self.datamap.remove(&id).map(|(_, data)| data);
        if data.is_some() {
            self.graph.remove_node(id);
        }

        data
    }

    /// Returns true if the graph contains the package.
    #[inline]
    pub fn contains_package<P>(&mut self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let id = self.keyid(&path);
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
        let pid = self.keyid(&parent);
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
        let pid = self.keyid(&parent);
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
        let pid = self.keyid(&parent);

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
    pub fn iter(&self) -> Iter<'_, Nodes<'_, u64>> {
        Iter {
            order: self.graph.nodes(),
            datamap: &self.datamap,
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            inner: self.datamap.iter_mut(),
        }
    }

    /// Returns an iterator of packages in topological sort order, with dependencies coming before
    /// dependents.
    #[inline]
    pub fn order<'g>(&'g self) -> Result<Iter<'g, vec::IntoIter<u64>>, CircularDependencyError> {
        let mut sorted = match algo::toposort(&self.graph, None) {
            Ok(v) => v,
            Err(cycle) => {
                let node_id = cycle.node_id();
                let (_, data) = self.datamap.get(&node_id).unwrap();
                return Err(CircularDependencyError(data.path.clone()).into());
            }
        };
        sorted.reverse();

        Ok(Iter {
            order: sorted.into_iter(),
            datamap: &self.datamap,
        })
    }

    #[inline]
    fn keyid<P: AsRef<Path>>(&self, path: P) -> u64 {
        let path = path.as_ref().to_path_buf();
        let mut s = DefaultHasher::new();
        path.hash(&mut s);
        s.finish()
    }
}

pub struct Iter<'g, I>
where
    I: Iterator<Item = u64>,
{
    order: I,
    datamap: &'g HashMap<u64, (PathBuf, PackageData)>,
}

impl<'g, I> Iterator for Iter<'g, I>
where
    I: Iterator<Item = u64>,
{
    type Item = (&'g Path, &'g PackageData);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.order.next()?;
        self.datamap
            .get(&id)
            .map(|(path, data)| (path.as_path(), data))
    }
}

pub struct IterMut<'g> {
    inner: hash_map::IterMut<'g, u64, (PathBuf, PackageData)>,
}

impl<'g> Iterator for IterMut<'g> {
    type Item = &'g mut PackageData;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, (_, v))| v)
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
