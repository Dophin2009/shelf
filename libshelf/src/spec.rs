use std::collections::HashMap;
use std::path::PathBuf;

pub use crate::tree::Tree;

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
}

// FIXME existing file replacement options
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

pub type Patterns = Vec<Pattern>;
pub type Pattern = String;

#[derive(Debug, Clone)]
pub enum LinkType {
    Link,
    Copy,
}

#[derive(Debug, Clone)]
pub struct TemplatedFile {
    pub src: PathBuf,
    pub dest: PathBuf,

    pub vars: Tree,

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

pub type HandlebarsPartials = HashMap<String, PathBuf>;

// FIXME partials & filters support
#[derive(Debug, Clone)]
pub struct LiquidTemplatedFile {}

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
    pub values: Tree,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TomlGeneratedFile {
    pub values: Tree,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JsonGeneratedFile {
    pub values: Tree,
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

pub type EnvMap = HashMap<String, String>;

#[derive(Debug, Clone)]
pub enum NonZeroExitBehavior {
    Error,
    Warn,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct FunHook {
    pub name: String,

    pub start: Option<PathBuf>,
    pub error_exit: Option<NonZeroExitBehavior>,
}

mod lua {
    use mlua::{Error as LuaError, FromLua, Value as LuaValue};

    use super::{LinkType, NonZeroExitBehavior};

    impl<'lua> FromLua<'lua> for LinkType {
        #[inline]
        fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
            match lua_value {
                LuaValue::String(s) => match s.to_str()? {
                    "link" => Ok(Self::Link),
                    "copy" => Ok(Self::Copy),
                    _ => conv_err(
                        LuaValue::String(s),
                        "LinkType",
                        r#"string ("link" or "copy")"#,
                    ),
                },
                _ => conv_err(lua_value, "LinkType", r#"string ("link" or "copy")"#),
            }
        }
    }

    impl<'lua> FromLua<'lua> for NonZeroExitBehavior {
        #[inline]
        fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
            match lua_value {
                LuaValue::String(s) => match s.to_str()? {
                    "error" => Ok(Self::Error),
                    "warn" => Ok(Self::Warn),
                    "ignore" => Ok(Self::Ignore),
                    _ => conv_err(
                        LuaValue::String(s),
                        "NonZeroExitBehavior",
                        r#"string ("error", "warn", or "ignore")"#,
                    ),
                },
                _ => conv_err(
                    lua_value,
                    "NonZeroExitBehavior",
                    r#"string ("error", "warn", or "ignore")"#,
                ),
            }
        }
    }

    fn conv_err<'lua, R>(value: LuaValue<'lua>, to: &'static str, should: &str) -> mlua::Result<R> {
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to,
            message: Some(format!("must be a {}", should)),
        })
    }
}
