use std::borrow::Cow;
use std::collections::VecDeque;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use mlua::{FromLua, Function, Lua, UserData, UserDataMethods, Variadic};
use uuid::Uuid;

use crate::error::MultipleError;
use crate::graph::{PackageData, PackageGraph};
use crate::pathutil::PathWrapper;
use crate::spec::{
    CmdHook, Dep, Directive, EmptyGeneratedFile, File, FunHook, GeneratedFile, GeneratedFileTyp,
    HandlebarsTemplatedFile, Hook, JsonGeneratedFile, LinkType, LiquidTemplatedFile,
    NonZeroExitBehavior, Patterns, RegularFile, Spec, StringGeneratedFile, TemplatedFile,
    TemplatedFileType, TomlGeneratedFile, TreeFile, YamlGeneratedFile,
};
use crate::tree::Tree;

static CONFIG_FILE: &str = "package.lua";

#[derive(Debug, thiserror::Error)]
#[error("couldn't load packages")]
pub struct LoaderError {
    #[from]
    pub errors: MultipleError<LoadError, PathWrapper>,
}

impl LoaderError {
    #[inline]
    pub fn new(errors: Vec<(LoadError, PathWrapper)>) -> Self {
        Self {
            errors: errors.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("couldn't open a file")]
    Open(io::Error),
    #[error("couldn't read a file")]
    Read(io::Error),
    #[error("couldn't execute Lua")]
    Lua(#[from] mlua::Error),
}

pub struct Loader {
    paths: Vec<PathWrapper>,

    pg: PackageGraph,
}

impl Loader {
    #[inline]
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            pg: PackageGraph::new(),
        }
    }

    #[inline]
    pub fn add<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path = path.as_ref().to_path_buf();
        let path = PathWrapper::from_cwd(path);
        self.paths.push(path);
        self
    }

    #[inline]
    pub fn load(&mut self) -> Result<&mut Self, LoaderError> {
        // FIXME option to fail-fast
        let load_it = self
            .paths
            .into_iter()
            .map(|p| (self.load_into_graph(&p), p));

        let errors: Vec<_> = load_it
            .filter_map(|(r, p)| r.err().map(|err| (err, p)))
            .collect();
        if errors.is_empty() {
            Ok(self)
        } else {
            Err(LoaderError::new(errors))
        }
    }

    #[inline]
    pub fn load_iter<'a, I>(&'a mut self) -> impl Iterator<Item = Result<(), LoadError>> + 'a {
        self.paths.iter().map(|p| {
            self.load_into_graph(&p)?;

            Ok(())
        })
    }

    #[inline]
    pub fn graph(&self) -> &PackageGraph {
        &self.pg
    }

    #[inline]
    pub fn to_graph(self) -> PackageGraph {
        self.pg
    }

    #[inline]
    fn insert_into_graph(&mut self, id: u64, package: PackageData) {
        // Insert package data into map.
        self.pg.map.insert(id, package);
        // Add graph node.
        self.pg.graph.add_node(id);
    }

    #[inline]
    fn load_deps(&mut self, deps: &[Dep]) -> Result<(), LoadError> {
        // Add nodes and edges for dependencies.
        let dep_it = deps.iter().map(|dep| -> Result<(), EmptyError> {
            // If given a relative path, make it absolute.
            let dep_path = dep.path.clone();
            let path = PathWrapper::from_cwd(dep_path);

            let dep_id = load_into_graph(pg, &path)?;
            pg.graph.add_edge(id, dep_id, ());

            Ok(())
        });

        let errors: Vec<_> = dep_it.filter_map(Result::err).collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LoadError::Multiple(errors.into()))
        }
    }
}

/// Iterator for loading package configurations; it returns instances of [`LoadIterSpecLoader`],
/// which can be used to actually load packages.
///
/// **Packages are loaded in an unspecified order. This means that any package may be read before
/// its dependencies.**
pub struct LoadIter {
    paths: VecDeque<(PathWrapper, Option<u64>)>,
    pg: PackageGraph,
}

impl Iterator for LoadIter {
    type Item = Result<LoadIterSpecLoader<'_>, LoadError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (parent_id, path) = self.paths.pop_front()?;

        Some(LoadIterSpecLoader::new(self, path, parent_id))
    }
}

impl<'a> LoadIter<'a> {
    /// Load a package from the given path into the graph and return its graph ID.
    #[inline]
    fn load_into_graph<'p, P>(&mut self, path: P) -> Result<u64, LoadError>
    where
        P: Into<Cow<'p, PathWrapper>>,
    {
        let path = Cow::from(path);

        // Check if this path has already been loaded.
        let id = hash_path(path.abs());
        if self.pg.map.get(&id).is_none() {
            // FIXME propogate error
            // Save current cwd.
            let cwd = env::current_dir().unwrap();
            // Work relative to the package root.
            env::set_current_dir(path.abs()).unwrap();

            // Load package data.
            let package = load_data(path)?;

            // Append dep paths to queue of paths to load.
            let deps = &package.data.deps;
            let dep_paths = deps
                .iter()
                .map(|Dep { path }| PathWrapper::from_cwd(path.clone()));
            self.paths.extend(dep_paths);

            // Insert the data into the graph.
            self.insert_into_graph(id, package);

            // Reload cwd.
            env::set_current_dir(&cwd).unwrap();
        }

        return Ok(id);
    }
}

pub struct LoadIterSpecLoader<'a> {
    iter: &'a mut LoadIter,
    sl: SpecLoader<SpecLoaderStateEmpty>,
    parent_id: Option<u64>,
}

impl<'a> LoadIterSpecLoader<'a> {
    #[inline]
    fn new(
        iter: &'a mut LoadIter,
        path: PathWrapper,
        parent_id: Option<u64>,
    ) -> Result<Self, LoadError> {
        Ok(Self {
            iter,
            sl: SpecLoader::new(path)?,
            parent_id,
        })
    }
}

pub struct SpecLoader<S>
where
    S: SpecLoaderState,
{
    path: PathWrapper,
    contents: String,
    lua: Lua,

    state: PhantomData<SpecLoaderState>,
}

trait SpecLoaderState {}
macro_rules! spec_loader_state {
    ($name:ident) => {
        pub struct $name;
        impl SpecLoaderState for $name {}
    };
    ($($names:ident),* $(,)?) => {
        $(spec_loader_state!($names);)*
    }
}

spec_loader_state!(
    SpecLoaderStateEmpty,
    SpecLoaderStateRead,
    SpecLoaderStateEvaled
);

impl SpecLoader<SpecLoaderStateEmpty> {
    #[inline]
    pub fn new(path: PathWrapper) -> Result<Self, LoadError> {
        let lua = Self::new_lua_inst()?;
        Ok(Self {
            path,
            contents: String::new(),
            lua,
            state: PhantomData::new(),
        })
    }

    #[inline]
    pub fn load<'a, P>(path: P) -> Result<PackageData, LoadError>
    where
        P: Into<Cow<'a, PathWrapper>>,
    {
        let path = Cow::from(path).into_owned();
        let loader = Self::new(path)?.read()?.eval()?;
        let pd = loader.to_package_data()?;
        Ok(pd)
    }

    /// Read the configuration contents.
    #[inline]
    pub fn read(self) -> Result<SpecLoader<SpecLoaderStateRead>, io::Error> {
        let config_path = self.path.join(CONFIG_FILE);
        let file = File::open(config_path.abs())?;
        file.read_to_string(&mut self.contents)?;

        Ok(SpecLoader::<SpecLoaderStateRead>::from(self))
    }
}

impl SpecLoader<SpecLoaderStateRead> {
    #[inline]
    pub fn eval(self) -> Result<SpecLoader<SpecLoaderStateEvaled>, mlua::Error> {
        let chunk = self.lua.load(&config_contents);
        chunk.exec()?;
        Ok(SpecLoader::<SpecLoaderStateEvaled>::from(self))
    }
}

impl SpecLoader<SpecLoaderStateEvaled> {
    #[inline]
    pub fn to_package_data(self) -> Result<PackageData, mlua::Error> {
        let package: SpecObject = lua.globals().get("pkg")?;
        Ok(PackageData {
            path: self.path,
            data: package.spec,
            lua: self.lua,
        })
    }

    #[inline]
    fn new_lua_inst() -> Result<Lua, mlua::Error> {
        #[cfg(not(feature = "unsafe"))]
        let lua = Lua::new();
        #[cfg(feature = "unsafe")]
        let lua = unsafe { Lua::unsafe_new() };

        lua.globals().set("pkg", SpecObject::new())?;
        lua.load(std::include_str!("globals.lua")).exec()?;

        Ok(lua)
    }
}

#[inline]
fn hash_path(path: &PathBuf) -> u64 {
    let mut s = DefaultHasher::new();
    path.hash(&mut s);
    s.finish()
}

#[derive(Debug, Clone)]
struct SpecObject {
    spec: Spec,
}

impl SpecObject {
    #[inline]
    fn new() -> Self {
        Self {
            spec: Spec {
                name: String::new(),
                deps: Vec::new(),
                directives: Vec::new(),
            },
        }
    }
}

impl UserData for SpecObject {
    #[inline]
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    #[inline]
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        macro_rules! method {
            ($name:expr; ($($arg:ident; $ty:ty),*); $drct:expr) => {
                #[allow(unused_parens)]
                methods.add_method_mut($name, |_, this, arg: ($($ty),*)| {
                    let ($($arg),*) = arg;
                    this.spec.directives.push($drct);
                    Ok(())
                });
            };
            ($name:expr; ($($arg:ident; $ty:ty),*); File; $drct:expr) => {
                method!($name; ($($arg; $ty),*); Directive::File($drct))
            };
            ($name:expr; ($($arg:ident; $ty:ty),*); Gen; $drct:expr) => {
                method!($name; ($($arg; $ty),*); Directive::File(File::Generated($drct)))
            };
            ($name:expr; ($($arg:ident; $ty:ty),*); Hook; $drct:expr) => {
                method!($name; ($($arg; $ty),*); Directive::Hook($drct))
            };
        }

        methods.add_method_mut("name", |_, this, name: String| {
            this.spec.name = name;
            Ok(())
        });

        methods.add_method_mut("dep", |_, this, paths: Variadic<String>| {
            this.spec
                .deps
                .extend(paths.into_iter().map(|p| Dep { path: p.into() }));
            Ok(())
        });

        method!("file"; (src; String, dest; Option<String>, link_type; Option<LinkType>, optional; Option<bool>);
        File; File::Regular(RegularFile {
            src: src.into(),
            dest: dest.map(Into::into),
            link_type: link_type.unwrap_or(LinkType::Link),
            optional: optional.unwrap_or(false)
        }));

        method!("tree"; (src; String, dest; Option<String>, link_type; Option<LinkType>,
                         globs; Option<Patterns>, ignore; Option<Patterns>, optional; Option<bool>);
        File; File::Tree(TreeFile {
            src: src.into(),
            dest: dest.map(Into::into),
            globs,
            ignore,
            link_type: link_type.unwrap_or(LinkType::Link),
            optional: optional.unwrap_or(false)
        }));

        method!("hbs"; (src; String, dest; String, vars; Tree, partials; HashMap<String, String>, optional; Option<bool>);
        File; {
            let partials = partials.into_iter().map(|(k, v)| (k, v.into())).collect();
            File::Templated(TemplatedFile {
                src: src.into(),
                dest: dest.into(),
                vars,
                typ: TemplatedFileType::Handlebars(HandlebarsTemplatedFile { partials }),
                optional: optional.unwrap_or(false)
            })
        });

        method!("liquid"; (src; String, dest; String, vars; Tree, optional; Option<bool>);
        File; File::Templated(TemplatedFile {
            src: src.into(),
            dest: dest.into(),
            vars,
            typ: TemplatedFileType::Liquid(LiquidTemplatedFile {}),
            optional: optional.unwrap_or(false)
        }));

        method!("empty"; (dest; String);
        Gen; GeneratedFile {
            dest: dest.into(), typ: GeneratedFileTyp::Empty(EmptyGeneratedFile)
        });
        method!("str"; (dest; String, contents; String);
        Gen; GeneratedFile {
            dest: dest.into(), typ: GeneratedFileTyp::String(StringGeneratedFile { contents })
        });
        method!("yaml"; (dest; String, values; Tree, header; Option<String>);
        Gen; GeneratedFile {
            dest: dest.into(), typ: GeneratedFileTyp::Yaml(YamlGeneratedFile { values, header })
        });
        method!("toml"; (dest; String, values; Tree, header; Option<String>);
        Gen; GeneratedFile {
            dest: dest.into(), typ: GeneratedFileTyp::Toml(TomlGeneratedFile { values, header })
        });
        method!("json"; (dest; String, values; Tree);
        Gen; GeneratedFile {
            dest: dest.into(), typ: GeneratedFileTyp::Json(JsonGeneratedFile { values })
        });

        method!("cmd"; (command; String, start; Option<String>, shell; Option<String>,
                        stdout; Option<bool>, stderr; Option<bool>,
                        clean_env; Option<bool>, env; Option<HashMap<String, String>>,
                        nonzero_exit; Option<NonZeroExitBehavior>);
        Hook; Hook::Cmd(CmdHook {
            command,
            start: start.map(Into::into),
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit
        }));

        methods.add_method_mut(
            "fn",
            |lua, this, arg: (Function, Option<String>, Option<NonZeroExitBehavior>)| {
                let (fun, start, error_exit) = arg;

                let name = Uuid::new_v4().to_string();
                lua.set_named_registry_value(&name, fun)?;

                let start = start.map(Into::into);

                let drct = Directive::Hook(Hook::Fun(FunHook {
                    name,
                    start,
                    error_exit,
                }));
                this.spec.directives.push(drct);
                Ok(())
            },
        );
    }
}
