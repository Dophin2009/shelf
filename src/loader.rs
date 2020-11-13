use crate::package::{
    Config, FileProcess, Hook, LinkType, Map, Package, TemplateEngine, TemplateProcess, Tree, Value,
};

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use mlua::{FromLua, Lua, Result as LuaResult, Value as LuaValue};
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

    fn lua_instance(&self) -> Lua {
        Lua::new()
    }

    fn load_package_data<P: AsRef<Path>>(&self, path: P) -> Result<Package> {
        let path = if path.as_ref().is_relative() {
            fs::canonicalize(&path)?
        } else {
            path.as_ref().into()
        };

        let config_path = path.join("package.lua");
        let configuration = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.to_string_lossy()))?;

        let lua = self.lua_instance();

        // Modify package.path
        let globals = lua.globals();
        let package_table = globals.get::<_, mlua::Table>("package")?;
        let old_packagepath = package_table.get::<_, String>("path")?;
        let new_packagepath = format!("{}/?.lua;{}", path.to_string_lossy(), old_packagepath);
        package_table.set("path", new_packagepath)?;

        // Load and evaluate lua code
        let chunk = lua.load(&configuration);
        let package: Package = chunk.eval().with_context(|| "Error in evalulating lua")?;

        // Restore package.path
        package_table.set("path", old_packagepath)?;

        Ok(package)
    }

    fn add_package<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let package = self.load_package_data(&path).with_context(|| {
            format!(
                "Failed to load package configuration: {}",
                path.as_ref().to_string_lossy()
            )
        })?;

        self.insert_package(PathBuf::from(path.as_ref()), package)
    }

    /// Add a package to the graph and map by absolute path.
    fn insert_package(&mut self, path: PathBuf, package: Package) -> Result<()> {
        let dependencies = package.config.dependencies.clone();

        let id = hash_path(&path);
        let existing = self.pg.map.insert(id, (path.clone(), package));
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

            self.insert_package(dep_path, dep)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct PackageGraph {
    /// Directional graph of package dependencies.
    graph: DiGraphMap<u64, ()>,
    /// Map storing the path and package
    map: HashMap<u64, (PathBuf, Package)>,
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

    pub fn package_map(&self) -> &HashMap<u64, (PathBuf, Package)> {
        &self.map
    }

    pub fn topological_order(&self) -> Result<impl Iterator<Item = &(PathBuf, Package)>> {
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

macro_rules! t_get {
    ($table:ident, $key:expr, $lua:ident) => {
        FromLua::from_lua($table.get($key)?, $lua)?;
    };
}

impl<'lua> FromLua<'lua> for Package {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let variables: Option<_> = t_get!(t, "variables", lua);
                let config = FromLua::from_lua(LuaValue::Table(t), lua)?;
                Ok(Self {
                    variables: variables.unwrap_or_else(Map::new),
                    config,
                })
            }
            // TODO: Properly handle invalid value.
            _ => panic!(),
        }
    }
}
impl<'lua> FromLua<'lua> for Config {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: Properly handle invalid values.
        match lua_value {
            LuaValue::Table(t) => {
                let name = t_get!(t, "name", lua);
                let dependencies = t_get!(t, "dependencies", lua);
                let default_link_type = t_get!(t, "link_type", lua);
                let ignore_patterns = t_get!(t, "ignore_patterns", lua);
                let files = t_get!(t, "files", lua);
                let template_files = t_get!(t, "templates", lua);
                let before_link = t_get!(t, "before_link", lua);
                let after_link = t_get!(t, "after_link", lua);
                let replace_files = t_get!(t, "replace_files", lua);
                let replace_directories = t_get!(t, "replace_dirs", lua);
                let trees = t_get!(t, "trees", lua);

                Ok(Self::new_optional(
                    name,
                    dependencies,
                    default_link_type,
                    ignore_patterns,
                    files,
                    template_files,
                    before_link,
                    after_link,
                    replace_files,
                    replace_directories,
                    trees,
                ))
            }
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for FileProcess {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: Properly handle invalid values.
        match lua_value {
            LuaValue::Table(t) => {
                let src = t_get!(t, "src", lua);
                let dest = t_get!(t, "dest", lua);
                let link_type = t_get!(t, "link_type", lua);
                let replace_files = t_get!(t, "replace_files", lua);
                let replace_directories = t_get!(t, "replace_dirs", lua);
                Ok(Self::new_optional(
                    src,
                    dest,
                    link_type,
                    replace_files,
                    replace_directories,
                ))
            }
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for TemplateProcess {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: Properly handle invalid values.
        match lua_value {
            LuaValue::Table(t) => {
                let src = t_get!(t, "src", lua);
                let dest = t_get!(t, "dest", lua);
                let engine = t_get!(t, "engine", lua);
                let replace_files = t_get!(t, "replace_files", lua);
                let replace_directories = t_get!(t, "replace_dirs", lua);
                Ok(Self::new_optional(
                    src,
                    dest,
                    engine,
                    replace_files,
                    replace_directories,
                ))
            }
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for TemplateEngine {
    // TODO: Properly handle invalid value.
    fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::String(s) => match s.to_str()?.to_lowercase().as_str() {
                "gtmpl" => Ok(TemplateEngine::Gtmpl),
                "tera" => Ok(TemplateEngine::Tera),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for Tree {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: Properly handle invalid values.
        match lua_value {
            LuaValue::Table(t) => {
                let path = t_get!(t, "path", lua);
                let default_link_type = t_get!(t, "link_type", lua);
                let ignore_patterns = t_get!(t, "ignore_patterns", lua);
                let replace_files = t_get!(t, "replace_files", lua);
                let replace_directories = t_get!(t, "replace_dirs", lua);
                Ok(Self::new_optional(
                    path,
                    default_link_type,
                    ignore_patterns,
                    replace_files,
                    replace_directories,
                ))
            }
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for LinkType {
    // TODO: Properly handle invalid value.
    fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::String(s) => match s.to_str()?.to_lowercase().as_str() {
                "link" => Ok(LinkType::Link),
                "copy" => Ok(LinkType::Copy),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for Hook {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: Properly handle invalid values.
        match lua_value {
            LuaValue::Table(t) => {
                let string = t_get!(t, "string", lua);
                let name = t_get!(t, "name", lua);
                Ok(Self::new(name, string))
            }
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for Map {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => Ok(Self {
                map: FromLua::from_lua(LuaValue::Table(t), lua)?,
            }),
            // TODO: Properly handle invalid value.
            _ => panic!(),
        }
    }
}

impl<'lua> FromLua<'lua> for Value {
    fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Boolean(b) => Ok(Value::Bool(b)),
            LuaValue::Integer(n) => Ok(Value::Integer(n)),
            LuaValue::String(s) => Ok(Value::String(s.to_str()?.into())),
            LuaValue::Number(n) => Ok(Value::Float(n)),
            LuaValue::Table(t) => {
                let hm = t.pairs().collect::<LuaResult<_>>()?;
                Ok(Value::Object(hm))
            }
            // TODO: Properly handle unsupported values.
            _ => panic!(),
        }
    }
}
