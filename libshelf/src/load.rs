use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::marker::PhantomData;

use mlua::Lua;

use self::specobject::SpecObject;
use crate::data::PackageData;
use crate::pathutil::PathWrapper;

static CONFIG_FILE: &str = "package.lua";

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("couldn't read a file")]
    Read(#[from] io::Error),
    #[error("couldn't execute Lua")]
    Lua(#[from] mlua::Error),
}

/// Loader for a package.
pub struct SpecLoader<S>
where
    S: SpecLoaderState,
{
    path: PathWrapper,
    contents: String,
    lua: Lua,

    state: PhantomData<S>,
}

pub type SpecLoaderEmpty = SpecLoader<SpecLoaderStateEmpty>;
pub type SpecLoaderRead = SpecLoader<SpecLoaderStateRead>;
pub type SpecLoaderEvaled = SpecLoader<SpecLoaderStateEvaled>;

/// Typestates for [`SpecLoader`].
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

impl SpecLoaderEmpty {
    /// Create a loader for the package at the given path.
    #[inline]
    pub fn new(path: PathWrapper) -> Result<Self, LoadError> {
        let lua = Self::new_lua_inst()?;
        Ok(Self {
            path,
            contents: String::new(),
            lua,
            state: PhantomData,
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

    /// Load the package, returning a [`PackageData`].
    #[inline]
    pub fn load<'a, P>(path: P) -> Result<PackageData, LoadError>
    where
        P: Into<Cow<'a, PathWrapper>>,
    {
        let path: Cow<'a, PathWrapper> = path.into();
        let loader = Self::new(path.into_owned())?;
        loader.finish()
    }

    /// Read the configuration contents.
    #[inline]
    pub fn read(self) -> Result<SpecLoaderRead, io::Error> {
        let config_path = self.path.join(CONFIG_FILE);
        let file = File::open(config_path.abs())?;
        file.read_to_string(&mut self.contents)?;

        Ok(SpecLoader {
            path: self.path,
            contents: self.contents,
            lua: self.lua,
            state: PhantomData,
        })
    }

    /// Load the package, returning a [`PackageData`].
    #[inline]
    pub fn finish(self) -> Result<PackageData, LoadError> {
        let res = self.read()?.eval()?.to_package_data()?;
        Ok(res)
    }
}

impl SpecLoaderRead {
    #[inline]
    pub fn eval(self) -> Result<SpecLoaderEvaled, mlua::Error> {
        // FIXME propogate error
        // Save current cwd.
        let cwd = env::current_dir().unwrap();
        // Work relative to the package root.
        env::set_current_dir(self.path.abs()).unwrap();

        // Eval lua.
        let chunk = self.lua.load(&self.contents);
        chunk.exec()?;

        // Reload cwd.
        env::set_current_dir(&cwd).unwrap();

        Ok(SpecLoader {
            path: self.path,
            contents: self.contents,
            lua: self.lua,
            state: PhantomData,
        })
    }

    /// Load the package, returning a [`PackageData`].
    #[inline]
    pub fn finish(self) -> Result<PackageData, LoadError> {
        let res = self.eval()?.to_package_data()?;
        Ok(res)
    }
}

impl SpecLoaderEvaled {
    #[inline]
    pub fn to_package_data(self) -> Result<PackageData, mlua::Error> {
        let package: SpecObject = self.lua.globals().get("pkg")?;
        Ok(PackageData {
            path: self.path,
            spec: package.spec,
            lua: self.lua,
        })
    }

    #[inline]
    pub fn finish(self) -> Result<PackageData, mlua::Error> {
        self.to_package_data()
    }
}

mod specobject {
    use std::collections::HashMap;

    use mlua::{Function, UserData, UserDataMethods, Variadic};
    use uuid::Uuid;

    use crate::spec::{
        CmdHook, Dep, Directive, EmptyGeneratedFile, File, FunHook, GeneratedFile,
        GeneratedFileTyp, HandlebarsTemplatedFile, Hook, JsonGeneratedFile, LinkType,
        LiquidTemplatedFile, NonZeroExitBehavior, Patterns, RegularFile, Spec, StringGeneratedFile,
        TemplatedFile, TemplatedFileType, TomlGeneratedFile, Tree, TreeFile, YamlGeneratedFile,
    };

    #[derive(Debug, Clone)]
    pub(super) struct SpecObject {
        pub(super) spec: Spec,
    }

    impl SpecObject {
        #[inline]
        pub fn new() -> Self {
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
}
