#[cfg(feature = "lua51")]
extern crate mlua51 as mlua;
#[cfg(feature = "lua52")]
extern crate mlua52 as mlua;
#[cfg(feature = "lua53")]
extern crate mlua53 as mlua;
#[cfg(feature = "lua54")]
extern crate mlua54 as mlua;
#[cfg(feature = "luajit")]
extern crate mluajit as mlua;

pub mod linker;
pub mod loader;

pub mod package;
pub mod template;

pub use linker::Linker;
pub use loader::Loader;
pub use package::*;
