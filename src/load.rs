use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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

#[derive(Debug, Clone)]
pub struct Loader {}

impl Loader {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    /// Loaded paths are not cleaned, and may be relative or absolute.
    #[inline]
    pub fn load(&self, path: impl AsRef<Path>) -> Result<PackageGraph> {
        self.load_multi(&[path])
    }

    #[inline]
    pub fn load_multi(&self, paths: &[impl AsRef<Path>]) -> Result<PackageGraph> {
        let mut s = LoaderState::new();
        paths
            .iter()
            .map(|p| {
                s.load(p).with_context(|| {
                    format!("Couldn't load package: {}", p.as_ref().to_string_lossy())
                })
            })
            .collect::<Result<_>>()?;

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

    #[inline]
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        // If given a relative path, make it absolute.
        let path = self.normalize_path(path)?;

        // Save current cwd.
        let cwd = self.cwd()?;
        // Work relative to the package root.
        env::set_current_dir(&path).with_context(|| "Couldn't change working directory.")?;

        let package = self.load_data(&path).with_context(|| {
            format!(
                "Couldn't load package configuration: {}",
                path.to_string_lossy()
            )
        })?;

        self.insert(package)?;

        env::set_current_dir(cwd).with_context(|| "Couldn't revert working directory")?;

        Ok(())
    }

    #[inline]
    fn insert(&mut self, package: PackageState) -> Result<()> {
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
            let dep = self.load_data(&dpath).with_context(|| {
                format!("Couldn't load dependency: {}", dep.path.to_string_lossy())
            })?;

            let dep_id = hash_path(&dpath);
            self.graph.graph.add_edge(id, dep_id, ());

            self.insert(dep)?;
        }

        Ok(())
    }

    #[inline]
    pub fn load_data(&mut self, path: impl AsRef<Path>) -> Result<PackageState> {
        let path = path.as_ref().to_path_buf();

        // Read the configuration contents.
        let config_path = path.join(CONFIG_FILE);
        let config_contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Couldn't read {}", config_path.to_string_lossy()))?;

        // Load and evaluate Lua code.
        let lua = self.lua_instance()?;
        let chunk = lua.load(&config_contents);
        chunk
            .exec()
            .with_context(|| "Something went wrong when executed lua")?;

        let pkg_data = lua
            .globals()
            .get("pkg")
            .with_context(|| "Global `pkg` wasn't set")?;
        let package: SpecObject = FromLua::from_lua(pkg_data, &lua)?;

        Ok(PackageState {
            path,
            data: package.spec,
            lua,
        })
    }

    #[inline]
    pub fn lua_instance(&self) -> Result<Lua> {
        #[cfg(not(feature = "unsafe"))]
        let lua = Lua::new();
        #[cfg(feature = "unsafe")]
        let lua = unsafe { Lua::unsafe_new() };

        lua.globals().set("pkg", SpecObject::new())?;
        lua.load(std::include_str!("globals.lua")).exec()?;

        Ok(lua)
    }

    #[inline]
    fn normalize_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let res = if path.as_ref().is_relative() {
            self.cwd()?.join(path)
        } else {
            path.as_ref().into()
        };
        Ok(res.clean())
    }

    #[inline]
    fn cwd(&self) -> Result<PathBuf> {
        env::current_dir().with_context(|| "Couldn't determine current directory")
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
