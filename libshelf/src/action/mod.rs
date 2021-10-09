mod command;
mod function;
mod generated;
mod link;
mod template;
mod tree;
mod write;

pub use self::command::*;
pub use self::function::*;
pub use self::generated::*;
pub use self::link::*;
pub use self::template::*;
pub use self::tree::*;
pub use self::write::*;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use mlua::Function;

use crate::op::{CopyOp, LinkOp, MkdirOp, Op, RmOp};
use crate::spec::{EnvMap, HandlebarsPartials, NonZeroExitBehavior, Patterns};
use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolve {
    fn resolve(self, opts: &ResolveOpts) -> ResolveResult;
}

pub type ResolveResult = Result<Resolution, ResolutionError>;

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("missing file: {0}")]
    FileMissing { pub path: PathBuf },
    #[error("couldn't read file metadata: {0}")]
    FileReadMetadata {
        pub path: PathBuf,
        pub err: io::Error,
    },
}

#[derive(Debug)]
pub enum Resolution {
    Done(DoneOutput),
    Skip(SkipReason),
}

#[derive(Debug)]
pub struct DoneOutput {
    ops: Vec<Op>,
    notices: Notice,
}

#[derive(Debug)]
pub enum Notice {
    Info(InfoNotice),
    Warn(WarnNotice),
}

#[derive(Debug)]
pub enum InfoNotice {
    ExistingSymlink {
        pub path: PathBuf,
        pub target: PathBuf,
    },
}

#[derive(Debug)]
pub enum WarnNotice {
    ManualChange { pub path: PathBuf },
    Overwrite { pub path: PathBuf },
}

#[derive(Debug)]
pub enum SkipReason {
    OptionalFileMissing { pub path: PathBuf },
}

pub enum Action<'lua> {
    Link(LinkAction),
    Write(WriteAction),
    Tree(TreeAction),
    Handlebars(HandlebarsAction),
    Liquid(LiquidAction),
    Yaml(YamlAction),
    Toml(TomlAction),
    Json(JsonAction),
    Command(CommandAction),
    Function(FunctionAction<'lua>),
}

impl<'a> Resolve for Action<'a> {
    #[inline]
    fn resolve(self, opts: &ResolveOpts) -> ResolveResult {
        match self {
            Self::Link(a) => a.resolve(opts),
            Self::Write(a) => a.resolve(opts),
            Self::Tree(a) => a.resolve(opts),
            Self::Handlebars(a) => a.resolve(opts),
            Self::Liquid(a) => a.resolve(opts),
            Self::Yaml(a) => a.resolve(opts),
            Self::Toml(a) => a.resolve(opts),
            Self::Json(a) => a.resolve(opts),
            Self::Command(a) => a.resolve(opts),
            Self::Function(a) => a.resolve(opts),
        }
    }
}
