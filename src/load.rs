use std::collections::{hash_map::DefaultHasher, HashMap};
use std::env;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use mlua::{FromLua, Function, Lua, UserData, UserDataMethods, Variadic};
use uuid::Uuid;

use crate::error::EmptyError;
use crate::graph::{PackageGraph, PackageState};
use crate::pathutil::PathWrapper;
use crate::spec::{
    CmdHook, Dep, Directive, EmptyGeneratedFile, File, FunHook, GeneratedFile, GeneratedFileTyp,
    HandlebarsTemplatedFile, Hook, JsonGeneratedFile, LinkType, LiquidTemplatedFile,
    NonZeroExitBehavior, Patterns, RegularFile, Spec, StringGeneratedFile, TemplatedFile,
    TemplatedFileType, TomlGeneratedFile, TreeFile, YamlGeneratedFile,
};
use crate::tree::Tree;

static CONFIG_FILE: &str = "package.lua";

macro_rules! lfail {
    ($res:expr) => {
        fail!($res, err => { render_err(err.into()) })
    };
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("couldn't read a file")]
    Read(PathWrapper, io::Error),
    #[error("Lua {}", .0)]
    Lua(#[from] mlua::Error),
}

/// Loaded paths are not cleaned, and may be relative or absolute.
#[inline]
pub fn load<P>(path: P) -> Result<PackageGraph, EmptyError>
where
    P: AsRef<Path>,
{
    load_multi(&[path])
}

#[inline]
pub fn load_multi(paths: &[impl AsRef<Path>]) -> Result<PackageGraph, EmptyError> {
    let mut s = LoaderState::new();

    // FIXME option to fail-fast
    let load_it = paths
        .iter()
        .map(|p| PathWrapper::from_cwd(p.as_ref().to_path_buf()))
        .map(|p| s.load(&p));
    if load_it.filter_map(|r| r.err()).count() != 0 {
        Err(EmptyError)
    } else {
        Ok(s.graph)
    }
}

struct LoaderState {
    graph: PackageGraph,
}

impl LoaderState {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: PackageGraph::new(),
        }
    }

    // FIXME needs lots of cleanup
    #[inline]
    pub fn load(&mut self, path: &PathWrapper) -> Result<(), EmptyError> {
        let _ = self.load_id(path)?;
        Ok(())
    }

    #[inline]
    fn load_id(&mut self, path: &PathWrapper) -> Result<u64, EmptyError> {
        tl_info!("Loading package at {[green]}", path.reld());

        // Check if this path has already been loaded.
        let id = hash_path(&path.abs().to_path_buf());
        if self.graph.map.get(&id).is_none() {
            // Save current cwd.
            let cwd = env::current_dir().unwrap();
            // Work relative to the package root.
            env::set_current_dir(path.abs()).unwrap();

            // Load package data.
            let package = self.load_data(path)?;

            // Insert the data into the graph.
            self.insert(id, package)?;

            // Reload cwd.
            env::set_current_dir(&cwd).unwrap();
        }

        return Ok(id);
    }

    #[inline]
    fn insert(&mut self, id: u64, package: PackageState) -> Result<(), EmptyError> {
        // Save deps for later.
        let deps = package.data.deps.clone();

        // Insert package data into map.
        self.graph.map.insert(id, package);
        // Add graph node.
        self.graph.graph.add_node(id);

        // Add nodes and edges for dependencies.
        sl_debug!("Resolving dependencies...");
        let dep_it = deps
            .iter()
            .map(|dep| -> Result<(), EmptyError> {
                // If given a relative path, make it absolute.
                let dep_path = dep.path.clone();
                let path = PathWrapper::from_cwd(dep_path);

                let dep_id = self.load_id(&path)?;
                self.graph.graph.add_edge(id, dep_id, ());

                Ok(())
            })
            .filter_map(Result::err);

        if dep_it.count() == 0 {
            Ok(())
        } else {
            Err(EmptyError)
        }
    }

    #[inline]
    pub fn load_data(&mut self, path: &PathWrapper) -> Result<PackageState, EmptyError> {
        // Read the configuration contents.
        sl_debug!("Trying to load package data...");
        let config_path = path.join(CONFIG_FILE);
        let config_contents =
            lfail!(fs::read_to_string(&config_path.abs())
                .map_err(|err| LoadError::Read(config_path, err)));

        // Load and evaluate Lua code.
        sl_debug!("Evalulating Lua code...");
        let lua = lfail!(lua_instance());
        let chunk = lua.load(&config_contents);
        lfail!(chunk.exec());

        // FIXME better error context
        sl_debug!("Retrieving package configuration object...");
        let pkg_data = lfail!(lua.globals().get("pkg").into());
        let package: SpecObject = lfail!(FromLua::from_lua(pkg_data, &lua));

        Ok(PackageState {
            path: path.clone(),
            data: package.spec,
            lua,
        })
    }
}

#[inline]
fn lua_instance() -> Result<Lua, mlua::Error> {
    #[cfg(not(feature = "unsafe"))]
    let lua = Lua::new();
    #[cfg(feature = "unsafe")]
    let lua = unsafe { Lua::unsafe_new() };

    lua.globals().set("pkg", SpecObject::new())?;
    lua.load(std::include_str!("globals.lua")).exec()?;

    Ok(lua)
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

#[inline]
fn render_err(err: LoadError) {
    fn render<E>(msg: impl fmt::Display, extra: impl fmt::Display, err: E)
    where
        E: std::error::Error,
    {
        sl_error!("{[red+bold]}: {[red]}\n      {}", msg, extra, err)
    }

    match err {
        LoadError::Read(path, err) => render("Couldn't read the package config", path.absd(), err),
        LoadError::Lua(err) => render("Couldn't evaluate Lua", "", err),
    };
}
