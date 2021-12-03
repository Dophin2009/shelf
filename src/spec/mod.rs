mod lua;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use crate::action::{
    object::{Object, Value as ObjectValue},
    template::hbs::HandlebarsPartials,
    tree::Patterns,
};
pub use crate::op::command::EnvMap;

#[derive(Debug, Clone)]
pub struct Spec {
    pub name: String,
    pub deps: Vec<Dep>,
    /// List of file link directives; order matters.
    pub directives: Vec<Directive>,
}

#[derive(Debug, Clone)]
pub enum Directive {
    File(File),
    Hook(Hook),
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum File {
    Regular(RegularFile),
    Tree(TreeFile),
    Templated(TemplatedFile),
    Generated(GeneratedFile),
    Dir(DirFile),
}

// FIXME existing file replacement options
// FIXME: permission
#[derive(Debug, Clone)]
pub struct RegularFile {
    pub src: PathBuf,
    /// Configuration can optionally specify a destination path relative to HOME.
    /// If none is provided, the relative src path will be used.
    pub dest: Option<PathBuf>,

    /// Files can be symlinked or copied to the destination.
    pub link_type: LinkType,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct TreeFile {
    pub src: PathBuf,
    pub dest: Option<PathBuf>,

    pub globs: Option<Patterns>,
    pub ignore: Option<Patterns>,

    pub link_type: LinkType,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub enum LinkType {
    Link,
    Copy,
}

// FIXME: permissions
#[derive(Debug, Clone)]
pub struct TemplatedFile {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub vars: Object,

    pub typ: TemplatedFileType,

    pub optional: bool,
}

// FIXME more template engine options
// FIXME pipe src content through function to get result
#[derive(Debug, Clone)]
pub enum TemplatedFileType {
    Handlebars(HandlebarsTemplatedFile),
    Liquid(LiquidTemplatedFile),
}

#[derive(Debug, Clone)]
pub struct HandlebarsTemplatedFile {
    pub partials: HandlebarsPartials,
}

// FIXME partials & filters support
#[derive(Debug, Clone)]
pub struct LiquidTemplatedFile {}

// FIXME: permissions
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub dest: PathBuf,
    pub typ: GeneratedFileTyp,
}

#[derive(Debug, Clone)]
pub enum GeneratedFileTyp {
    Empty(EmptyGeneratedFile),
    String(StringGeneratedFile),
    Yaml(YamlGeneratedFile),
    Toml(TomlGeneratedFile),
    Json(JsonGeneratedFile),
}

#[derive(Debug, Clone)]
pub struct EmptyGeneratedFile;

#[derive(Debug, Clone)]
pub struct StringGeneratedFile {
    pub contents: String,
}

#[derive(Debug, Clone)]
pub struct YamlGeneratedFile {
    pub values: Object,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TomlGeneratedFile {
    pub values: Object,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JsonGeneratedFile {
    pub values: Object,
}

// TODO: permissions
#[derive(Debug, Clone)]
pub struct DirFile {
    pub dest: PathBuf,
    pub parents: bool,
}

#[derive(Debug, Clone)]
pub enum Hook {
    Cmd(CmdHook),
    Fun(FunHook),
}

// FIXME optional env variables?
#[derive(Debug, Clone)]
pub struct CmdHook {
    pub command: String,

    pub start: Option<PathBuf>,
    pub shell: Option<String>,

    pub stdout: Option<bool>,
    pub stderr: Option<bool>,

    pub clean_env: Option<bool>,
    pub env: Option<EnvMap>,

    pub nonzero_exit: Option<NonZeroExitBehavior>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum NonZeroExitBehavior {
    Error,
    Warn,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct FunHook {
    pub name: String,

    pub start: Option<PathBuf>,
    pub nonzero_exit: Option<NonZeroExitBehavior>,
}
