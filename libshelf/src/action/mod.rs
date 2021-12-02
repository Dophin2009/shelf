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

use std::path::PathBuf;

use crate::op::Op;

// Re-export main trait.
pub use self::resolve::Resolve;
// Re-export action types.
pub use self::command::CommandAction;
pub use self::function::FunctionAction;
pub use self::generated::{JsonAction, TomlAction, YamlAction};
pub use self::link::LinkAction;
pub use self::mkdir::MkdirAction;
pub use self::template::{HandlebarsAction, LiquidAction};
pub use self::tree::TreeAction;
pub use self::write::WriteAction;

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
