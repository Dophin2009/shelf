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

use crate::graph::{PackageGraph, PackageState};
use crate::spec::{
    CmdHook, Dep, Directive, EmptyGeneratedFile, File, FunHook, GeneratedFile, GeneratedFileTyp,
    HandlebarsTemplatedFile, Hook, JsonGeneratedFile, LinkType, LiquidTemplatedFile, Patterns,
    RegularFile, Spec, StringGeneratedFile, TemplatedFile, TemplatedFileType, TomlGeneratedFile,
    TreeFile, YamlGeneratedFile,
};
use crate::tree::Tree;

static CONFIG_FILE: &str = "package.lua";

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("couldn't read a file")]
    Read(io::Error),
    #[error("couldn't determine the current working directory")]
    Cwd(io::Error),
    #[error("couldn't change the current working directory")]
    Chdir(io::Error),
    #[error("lua error")]
    Lua(#[from] mlua::Error),
}

/// Loaded paths are not cleaned, and may be relative or absolute.
#[inline]
pub fn load(path: impl AsRef<Path>) -> Result<PackageGraph, LoadError> {
    load_multi(&[path])
}

#[inline]
pub fn load_multi(paths: &[impl AsRef<Path>]) -> Result<PackageGraph, LoadError> {
    let mut s = LoaderState::new();
    paths.iter().map(|p| s.load(p)).collect::<Result<_, _>>()?;

    Ok(s.graph)
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

    #[inline]
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<(), LoadError> {
        // If given a relative path, make it absolute.
        let path = self.normalize_path(path)?;

        // Save current cwd.
        let cwd = env::current_dir().map_err(LoadError::Cwd)?;
        // Work relative to the package root.
        env::set_current_dir(&path).map_err(LoadError::Chdir)?;

        // Load package data.
        let package = self.load_data(&path)?;
        self.insert(package)?;

        // Reload cwd.
        env::set_current_dir(cwd).map_err(LoadError::Chdir)?;

        Ok(())
    }

    #[inline]
    fn insert(&mut self, package: PackageState) -> Result<(), LoadError> {
        // If already added, stop.
        let path = &package.path;
        let deps = package.data.deps.clone();
        let id = hash_path(path);
        if self.graph.map.insert(id, package).is_some() {
            return Ok(());
        }
        self.graph.graph.add_node(id);

        for dep in deps {
            let dpath = self.normalize_path(&dep.path)?;
            // FIXME more error context
            let dep = self.load_data(&dpath)?;

            let dep_id = hash_path(&dpath);
            self.graph.graph.add_edge(id, dep_id, ());

            self.insert(dep)?;
        }

        Ok(())
    }

    #[inline]
    pub fn load_data(&mut self, path: impl AsRef<Path>) -> Result<PackageState, LoadError> {
        let path = path.as_ref().to_path_buf();

        // Read the configuration contents.
        let config_path = path.join(CONFIG_FILE);
        let config_contents = fs::read_to_string(&config_path).map_err(LoadError::Read)?;

        // Load and evaluate Lua code.
        let lua = self.lua_instance()?;
        let chunk = lua.load(&config_contents);
        chunk.exec()?;

        // FIXME better error context
        let pkg_data = lua.globals().get("pkg")?;
        let package: SpecObject = FromLua::from_lua(pkg_data, &lua)?;

        Ok(PackageState {
            path,
            data: package.spec,
            lua,
        })
    }

    #[inline]
    pub fn lua_instance(&self) -> Result<Lua, LoadError> {
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

        method!("cmd"; (command; String, quiet; Option<bool>, start; Option<String>, shell; Option<String>);
        Hook; Hook::Cmd(CmdHook { command, quiet, start: start.map(Into::into), shell })
        );

        methods.add_method_mut("fn", |lua, this, arg: (Function, Option<bool>)| {
            let (fun, quiet) = arg;

            let name = Uuid::new_v4().to_string();
            lua.set_named_registry_value(&name, fun)?;

            let drct = Directive::Hook(Hook::Fun(FunHook { name, quiet }));
            this.spec.directives.push(drct);
            Ok(())
        });
    }
}
