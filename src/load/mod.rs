mod specobject;

use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use mlua::Lua;

use crate::graph::PackageData;

use self::specobject::SpecObject;

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
    path: PathBuf,
    contents: String,
    lua: Lua,

    state: PhantomData<S>,
}

pub type SpecLoaderEmpty = SpecLoader<SpecLoaderStateEmpty>;
pub type SpecLoaderRead = SpecLoader<SpecLoaderStateRead>;
pub type SpecLoaderEvaled = SpecLoader<SpecLoaderStateEvaled>;

/// Typestates for [`SpecLoader`].
pub trait SpecLoaderState: specobject::SpecLoaderState {}
macro_rules! spec_loader_state {
    ($name:ident) => {
        pub struct $name;
        impl specobject::SpecLoaderState for $name {}
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
    pub fn new<P>(path: P) -> Result<Self, LoadError>
    where
        P: AsRef<Path>,
    {
        let lua = Self::lua_instance()?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            contents: String::new(),
            lua,
            state: PhantomData,
        })
    }

    #[inline]
    fn lua_instance() -> Result<Lua, mlua::Error> {
        #[cfg(not(feature = "lua-unsafe"))]
        let lua = Lua::new();
        #[cfg(feature = "lua-unsafe")]
        let lua = unsafe { Lua::unsafe_new() };

        lua.globals().set("pkg", SpecObject::new())?;
        lua.load(std::include_str!("globals.lua")).exec()?;

        Ok(lua)
    }

    /// Load the package, returning a [`PackageData`].
    #[inline]
    pub fn load<P>(path: P) -> Result<PackageData, LoadError>
    where
        P: AsRef<Path>,
    {
        let loader = Self::new(path)?;
        loader.finish()
    }

    /// Read the configuration contents.
    #[inline]
    pub fn read(mut self) -> Result<SpecLoaderRead, io::Error> {
        let config_path = self.path.join(CONFIG_FILE);
        let mut file = File::open(config_path)?;
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
        env::set_current_dir(&self.path).unwrap();

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
