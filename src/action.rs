use std::path::PathBuf;

use anyhow::Result;
use mlua::Function;

pub enum Action<'lua> {
    LinkFile(LinkFileAction),
    WriteFile(WriteFileAction),
    RunCommand(RunCommandAction),
    RunFunction(RunFunctionAction<'lua>),
}

impl<'lua> Action<'lua> {
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
    pub function: Function<'lua>,
}
