pub mod error;
pub mod resolve;

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

pub use self::resolve::Resolve;

use std::path::PathBuf;

use crate::op::Op;

#[derive(Debug, Clone)]
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

impl<'lua> Resolve<'lua> for Action<'lua> {
    type Error = ResolutionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let res: Res<'lua> = match self {
            Self::Link(a) => a.resolve(opts)?,
            Self::Write(a) => a.resolve(opts)?,
            Self::Tree(a) => a.resolve(opts)?,
            Self::Handlebars(a) => a.resolve(opts)?,
            Self::Liquid(a) => a.resolve(opts)?,
            Self::Yaml(a) => a.resolve(opts)?,
            Self::Toml(a) => a.resolve(opts)?,
            Self::Json(a) => a.resolve(opts)?,
            Self::Mkdir(a) => a.resolve(opts)?,
            Self::Command(a) => a.resolve(opts)?,
            Self::Function(a) => a.resolve(opts)?,
        };
        Ok(res)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("link action resolution error")]
    Link(#[from] Error),
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
