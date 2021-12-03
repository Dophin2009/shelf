pub mod error;

pub mod object;

pub mod command;
pub mod function;
pub mod generated;
pub mod link;
pub mod mkdir;
pub mod template;
pub mod tree;
pub mod write;

// Re-export action types.
pub use self::command::CommandAction;
pub use self::function::FunctionAction;
pub use self::generated::{JsonAction, TomlAction, YamlAction};
pub use self::link::LinkAction;
pub use self::mkdir::MkdirAction;
pub use self::template::{HandlebarsAction, LiquidAction};
pub use self::tree::TreeAction;
pub use self::write::WriteAction;

pub trait Resolve {
    type Output;

    fn resolve(&self) -> Self::Output;
}

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
    Link(#[from] self::link::Error),
    #[error("handlebars action resolution error")]
    Handlebars(#[from] self::template::hbs::Error),
    #[error("liquid action resolution error")]
    Liquid(#[from] self::template::liquid::Error),
    #[error("yaml action resolution error")]
    Yaml(#[from] self::generated::yaml::Error),
    #[error("toml action resolution error")]
    Toml(#[from] self::generated::toml::Error),
    #[error("json action resolution error")]
    Json(#[from] self::generated::json::Error),
    #[error("command action resolution error")]
    Command(#[from] self::command::Error),
    #[error("function action resolution error")]
    Function(#[from] self::function::Error),
}
