use core::fmt;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

use mlua::{FromLua, Function, Lua, UserData, UserDataMethods, Variadic};
use path_clean::PathClean;
use uuid::Uuid;

use crate::error::EmptyError;
use crate::format::{self, errored, style, sublevel, toplevel};
use crate::graph::{PackageGraph, PackageState};
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
    Read(PathBuf, io::Error),
    #[error("couldn't determine the current working directory")]
    Cwd(io::Error),
    #[error("couldn't change the current working directory")]
    Chdir(PathBuf, io::Error),
    #[error("Lua {}", .0)]
    Lua(#[from] mlua::Error),
}

/// Loaded paths are not cleaned, and may be relative or absolute.
#[inline]
pub fn load(path: impl AsRef<Path>) -> Result<PackageGraph, EmptyError> {
    load_multi(&[path])
}

#[inline]
pub fn load_multi(paths: &[impl AsRef<Path>]) -> Result<PackageGraph, EmptyError> {
    let mut s = LoaderState::new();

    // FIXME option to fail-fast
    let load_it = paths.iter().map(|p| s.load(p));
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
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), EmptyError> {
        let _ = self.load_id(path)?;
        Ok(())
    }

    #[inline]
    fn load_id(&mut self, path: impl AsRef<Path>) -> Result<u64, EmptyError> {
        let path = path.as_ref().to_path_buf();

        toplevel::info(&format!(
            "Loading package at {}",
            format::filepath(path.display())
        ));

        // Check if this path has already been loaded.
        let id = hash_path(&path);
        if self.graph.map.get(&id).is_none() {
            // Save current cwd.
            let cwd = lfail!(env::current_dir().map_err(LoadError::Cwd));
            // Work relative to the package root.
            lfail!(env::set_current_dir(&path).map_err(|err| LoadError::Chdir(path.clone(), err)));

            // Load package data.
            let package = self.load_data(&path)?;

            // Insert the data into the graph.
            self.insert(id, package)?;

            // Reload cwd.
            lfail!(env::set_current_dir(&cwd).map_err(|err| LoadError::Chdir(cwd, err)));
        }

        return Ok(id);
    }

    #[inline]
    fn insert(&mut self, id: u64, package: PackageState) -> Result<(), EmptyError> {
        // Insert package data into map.
        let deps = package.data.deps.clone();
        self.graph.map.insert(id, package);

        // Add graph node.
        self.graph.graph.add_node(id);

        // Add nodes and edges for dependencies.
        sublevel::debug("Resolving dependencies...");
        let dep_it = deps
            .iter()
            .map(|dep| {
                // If given a relative path, make it absolute.
                let path = lfail!(self.normalize_path(&dep.path));

                let dep_id = self.load_id(path)?;
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
    pub fn load_data(&mut self, path: impl AsRef<Path>) -> Result<PackageState, EmptyError> {
        sublevel::debug("Trying to load package data...");
        let path = path.as_ref().to_path_buf();

        // Read the configuration contents.
        let config_path = path.join(CONFIG_FILE);
        let config_contents = lfail!(
            fs::read_to_string(&config_path).map_err(|err| LoadError::Read(config_path, err))
        );

        // Load and evaluate Lua code.
        sublevel::debug("Evalulating Lua code...");
        let lua = lfail!(self.lua_instance());
        let chunk = lua.load(&config_contents);
        lfail!(chunk.exec());

        // FIXME better error context
        sublevel::debug("Retrieving package configuration object...");
        let pkg_data = lfail!(lua.globals().get("pkg").into());
        let package: SpecObject = lfail!(FromLua::from_lua(pkg_data, &lua));

        Ok(PackageState {
            path,
            data: package.spec,
            lua,
        })
    }

    #[inline]
    fn lua_instance(&self) -> Result<Lua, mlua::Error> {
        #[cfg(not(feature = "unsafe"))]
        let lua = Lua::new();
        #[cfg(feature = "unsafe")]
        let lua = unsafe { Lua::unsafe_new() };

        lua.globals().set("pkg", SpecObject::new())?;
        lua.load(std::include_str!("globals.lua")).exec()?;

        Ok(lua)
    }

    #[inline]
    fn normalize_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, LoadError> {
        let res = if path.as_ref().is_relative() {
            env::current_dir().map_err(LoadError::Cwd)?.join(path)
        } else {
            path.as_ref().into()
        };

        Ok(res.clean())
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
            |lua, this, arg: (Function, Option<NonZeroExitBehavior>)| {
                let (fun, error_exit) = arg;

                let name = Uuid::new_v4().to_string();
                lua.set_named_registry_value(&name, fun)?;

                let drct = Directive::Hook(Hook::Fun(FunHook { name, error_exit }));
                this.spec.directives.push(drct);
                Ok(())
            },
        );
    }
}

#[inline]
fn render_err(err: LoadError) {
    fn render<E>(msg: impl fmt::Display, extra: impl fmt::Display, err: E) -> String
    where
        E: std::error::Error,
    {
        format!(
            "{} {}\n      {}",
            style(format!("{}:", msg)).bold().red(),
            style(extra).red(),
            err
        )
    }

    let rendered = match err {
        LoadError::Read(path, err) => {
            render("Couldn't read the package config", path.display(), err)
        }
        LoadError::Cwd(err) => render("Couldn't determine the current directory", "", err),
        LoadError::Chdir(path, err) => {
            render("Couldn't change current directory", path.display(), err)
        }
        LoadError::Lua(err) => render("Couldn't evaluate Lua", "", err),
    };

    errored::error(&rendered);
}
