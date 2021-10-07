pub mod command;
pub mod function;
pub mod generated;
pub mod link;
pub mod template;
pub mod tree;
pub mod write;

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use mlua::Function;

use crate::cache::{Cache, FileTyp};
use crate::op::LinkOp;
use crate::op::RmOp;
use crate::op::{CopyOp, MkdirOp, Op};
use crate::spec::{EnvMap, HandlebarsPartials, NonZeroExitBehavior, Patterns};
use crate::tree::Tree;

use self::command::CommandAction;
use self::function::FunctionAction;
use self::generated::{JsonAction, TomlAction, YamlAction};
use self::link::LinkAction;
use self::template::{HandlebarsAction, LiquidAction};
use self::tree::TreeAction;
use self::write::WriteAction;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolve {
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache;
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
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        match self {
            Self::Link(a) => a.resolve(opts, cache),
            Self::Write(a) => a.resolve(opts, cache),
            Self::Tree(a) => a.resolve(opts, cache),
            Self::Handlebars(a) => a.resolve(opts, cache),
            Self::Liquid(a) => a.resolve(opts, cache),
            Self::Yaml(a) => a.resolve(opts, cache),
            Self::Toml(a) => a.resolve(opts, cache),
            Self::Json(a) => a.resolve(opts, cache),
            Self::Command(a) => a.resolve(opts, cache),
            Self::Function(a) => a.resolve(opts, cache),
        }
    }
}

// macro_rules! log_skip {
// () => {
// sl_debug!("")
// };
// ($format_str:literal $(, $arg:expr)* $(,)?) => {
// log_skip!([$format_str] $(, $arg )*)
// };
// ([$($format_str:literal),+ $(,)?] $(, $arg:expr)* $(,)?) => {
// sl_debug!(["{$yellow}Skipping...{/$} ", $($format_str),+] $(, $arg)*)
// };
// }

// #[inline]
// fn log_miss(path: &PathWrapper) {
// sl_error!("{$red}Failed!{/$} Missing file: {[green]}", path.absd());
// }
