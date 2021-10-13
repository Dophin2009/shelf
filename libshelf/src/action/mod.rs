pub mod error;

mod command;
mod function;
mod generated;
mod link;
mod mkdir;
mod template;
mod tree;
mod write;

// FIXME: re-export errors in separate mod
pub use self::command::*;
pub use self::function::*;
pub use self::generated::*;
pub use self::link::*;
pub use self::mkdir::*;
pub use self::template::*;
pub use self::tree::*;
pub use self::write::*;

use std::path::PathBuf;

use crate::op::Op;

#[derive(Debug, Clone)]
pub struct ResolveOpts {}

pub trait Resolve {
    type Error;

    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution, Self::Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("link action resolution error")]
    Link(#[from] LinkActionError),
    #[error("write action resolution error")]
    Write(#[from] WriteActionError),
    #[error("tree action resolution error")]
    Tree(#[from] TreeActionError),
    #[error("handlebars action resolution error")]
    Handlebars(#[from] HandlebarsActionError),
    #[error("liquid action resolution error")]
    Liquid(#[from] LiquidActionError),
    #[error("yaml action resolution error")]
    Yaml(#[from] YamlActionError),
    #[error("toml action resolution error")]
    Toml(#[from] TomlActionError),
    #[error("json action resolution error")]
    Json(#[from] JsonActionError),
    #[error("command action resolution error")]
    Command(#[from] CommandActionError),
    #[error("function action resolution error")]
    Function(#[from] FunctionActionError),
}

#[derive(Debug)]
pub enum Resolution<'lua> {
    Done(DoneOutput<'lua>),
    Skip(SkipReason),
    Multiple(Vec<Resolution<'lua>>),
}

#[derive(Debug)]
pub struct DoneOutput<'lua> {
    pub ops: Vec<Op<'lua>>,
    pub notices: Vec<Notice>,
}

impl<'lua> DoneOutput<'lua> {
    #[inline]
    pub fn new(ops: Vec<Op<'lua>>, notices: Vec<Notice>) -> Self {
        Self { ops, notices }
    }

    #[inline]
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new())
    }
}

impl<'lua> Default for DoneOutput<'lua> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug)]
pub enum Notice {
    Info(InfoNotice),
    Warn(WarnNotice),
}

#[derive(Debug)]
pub enum InfoNotice {
    ExistingSymlink { path: PathBuf, target: PathBuf },
}

#[derive(Debug)]
pub enum WarnNotice {
    ManualChange { path: PathBuf },
    Overwrite { path: PathBuf },
}

// TODO: split up as with ResolutionError?
#[derive(Debug)]
pub enum SkipReason {
    OptionalFileMissing { path: PathBuf },
    DestinationExists { path: PathBuf },
}

#[derive(Debug)]
pub enum Action<'lua> {
    Link(LinkAction),
    Write(WriteAction),
    Tree(TreeAction),
    Handlebars(HandlebarsAction),
    Liquid(LiquidAction),
    Yaml(YamlAction),
    Toml(TomlAction),
    Json(JsonAction),
    Mkdir(MkdirAction),
    Command(CommandAction),
    Function(FunctionAction<'lua>),
}

impl<'lua> Resolve for Action<'lua> {
    type Error = ResolutionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Resolution<'lua>, Self::Error> {
        let res: Resolution<'lua> = match self {
            Self::Link(a) => todo!(),
            Self::Write(a) => todo!(),
            Self::Tree(a) => todo!(),
            Self::Handlebars(a) => todo!(),
            Self::Liquid(a) => todo!(),
            Self::Yaml(a) => todo!(),
            Self::Toml(a) => todo!(),
            Self::Json(a) => todo!(),
            Self::Mkdir(a) => todo!(),
            Self::Command(a) => todo!(),
            Self::Function(a) => a.resolve(opts)?,
        };
        Ok(res)
    }
}
