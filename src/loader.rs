use crate::package::Package;

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use mlua::{Lua, Value as LuaValue};
use petgraph::algo;
use petgraph::graphmap::DiGraphMap;

pub struct Loader {}

impl Loader {
    pub fn new() -> Self {
        Self {}
    }

    /// Load a package and all its dependencies into a package graph.
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<PackageGraph> {
        self.load_multi(&[path])
    }

    pub fn load_multi(&self, paths: &[impl AsRef<Path>]) -> Result<PackageGraph> {
        let mut state = LoaderState::new();

        paths
            .iter()
            .map(|p| {
                state.add_package(p).with_context(|| {
                    format!("Failed to load package: {}", p.as_ref().to_string_lossy())
                })
            })
            .collect::<Result<_>>()?;
        Ok(state.pg)
    }
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}

struct LoaderState {
    pg: PackageGraph,
}

impl LoaderState {
    fn new() -> Self {
        Self {
            pg: PackageGraph::new(),
        }
    }

    fn lua_instance(&self) -> Result<Lua> {
        fn empty_vec<'a>() -> Vec<LuaValue<'a>> {
            vec![]
        }
        let lua = Lua::new();

        // Add global `pkg` to be modified
        let pkg = lua.create_table()?;

        let default_tree = lua.create_table()?;
        default_tree.set("path", "tree")?;
        default_tree.set("link_type", "link")?;
        default_tree.set("ignore", empty_vec())?;
        default_tree.set("replace_files", LuaValue::Nil)?;
        default_tree.set("replace_dirs", LuaValue::Nil)?;

        let files = lua.create_table()?;
        files.set("trees", vec![default_tree])?;
        files.set("extra", empty_vec())?;
        files.set("templates", empty_vec())?;
        files.set("replace_files", true)?;
        files.set("replace_dirs", false)?;

        let hooks = lua.create_table()?;
        hooks.set("pre", empty_vec())?;
        hooks.set("post", empty_vec())?;

        pkg.set("dependencies", empty_vec())?;
        pkg.set("files", files)?;
        pkg.set("hooks", hooks)?;
        pkg.set("variables", lua.create_table()?)?;

        {
            let globals = lua.globals();
            globals.set("pkg", pkg)?;
        }

        Ok(lua)
    }

    fn load_package_data<P: AsRef<Path>>(&self, path: P) -> Result<PackageState> {
        let path = if path.as_ref().is_relative() {
            fs::canonicalize(&path)?
        } else {
            path.as_ref().into()
        };

        let config_path = path.join("package.lua");
        let configuration = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.to_string_lossy()))?;

        // Load and evaluate lua code
        let lua = self.lua_instance()?;
        let chunk = lua.load(&configuration);
        chunk.exec().with_context(|| "Error in executing lua")?;

        let package = lua.globals().get("pkg")?;
        Ok(PackageState {
            path,
            data: package,
            lua,
        })
    }

    fn add_package<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let package = self.load_package_data(&path).with_context(|| {
            format!(
                "Failed to load package configuration: {}",
                path.as_ref().to_string_lossy()
            )
        })?;

        self.insert_package(package)
    }

    /// Add a package to the graph and map by absolute path.
    fn insert_package(&mut self, package: PackageState) -> Result<()> {
        let dependencies = package.data.dependencies.clone();

        let path = &package.path.clone();
        let id = hash_path(path);
        let existing = self.pg.map.insert(id, package);
        if existing.is_some() {
            return Ok(());
        }

        self.pg.graph.add_node(id);

        // Add dependencies of the package.
        for dep_path_rel in &dependencies {
            let dep_path_abs = path.join(dep_path_rel);
            let dep_path = fs::canonicalize(dep_path_abs)
                .with_context(|| format!("Failed to resolve dependency: {}", dep_path_rel))?;
            let dep = self.load_package_data(&dep_path).with_context(|| {
                format!("Failed to load dependency: {}", dep_path.to_string_lossy())
            })?;

            let dep_id = hash_path(&dep_path);
            self.pg.graph.add_edge(id, dep_id, ());

            self.insert_package(dep)?;
        }

        Ok(())
    }
}

pub struct PackageState {
    pub path: PathBuf,
    pub data: Package,
    pub lua: Lua,
}

pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<u64, ()>,
    /// Map storing the path and package
    map: HashMap<u64, PackageState>,
}

impl PackageGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::new(),
            map: HashMap::new(),
        }
    }

    pub fn graph(&self) -> &DiGraphMap<u64, ()> {
        &self.graph
    }

    pub fn package_map(&self) -> &HashMap<u64, PackageState> {
        &self.map
    }

    pub fn topological_order(&self) -> Result<impl Iterator<Item = &PackageState>> {
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

impl Default for PackageGraph {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_path(path: &PathBuf) -> u64 {
    let mut s = DefaultHasher::new();
    path.hash(&mut s);
    s.finish()
}
