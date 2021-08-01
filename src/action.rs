use std::path::PathBuf;

use anyhow::Result;
use mlua::Lua;

pub enum Action<'lua> {
    LinkFile(LinkFileAction),
    WriteFile(WriteFileAction),
    RunCommand(RunCommandAction),
    RunFunction(RunFunctionAction<'lua>),
}

impl<'p> Action<'p> {
    #[inline]
    pub fn resolve(&self) -> Result<()> {
        Ok(())
    }
}

pub struct LinkFileAction {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub copy: bool,
}

pub struct WriteFileAction {
    pub dest: PathBuf,
    pub contents: String,
}

pub struct RunCommandAction {
    pub command: String,

    pub quiet: bool,
    pub start: PathBuf,
    pub shell: String,
}

pub struct RunFunctionAction<'lua> {
    pub name: String,
    pub lua: &'lua Lua,
}
