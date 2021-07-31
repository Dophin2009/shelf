use std::collections::HashMap;
use std::path::PathBuf;

use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct Spec {
    pub name: String,
    /// List of file link directives; order matters.
    pub directives: Vec<Directive>,
}

#[derive(Debug, Clone)]
pub enum Directive {
    Dep(Dep),
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
}

#[derive(Debug, Clone)]
pub struct TreeFile {
    pub src: PathBuf,
    pub dest: Option<PathBuf>,

    pub link_type: LinkType,
    pub ignore: IgnorePatterns,
}

pub type IgnorePatterns = Vec<IgnorePattern>;
pub type IgnorePattern = String;

#[derive(Debug, Clone)]
pub enum LinkType {
    Link,
    Copy,
}

#[derive(Debug, Clone)]
pub struct TemplatedFile {
    pub src: PathBuf,
    pub dest: PathBuf,

    /// Optional set of variables to use for this template; globals will not be used if this is
    /// set.
    pub vars: Option<Tree>,

    pub typ: TemplatedFileType,
}

// FIXME more template engine options
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

#[derive(Debug, Clone)]
pub struct CmdHook {
    pub command: String,

    pub quiet: Option<bool>,
    pub start: Option<PathBuf>,
    pub shell: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FunHook {
    pub name: String,

    pub quiet: Option<bool>,
}

mod lua {
    use mlua::{Error as LuaError, FromLua, Value as LuaValue};

    use super::LinkType;

    impl<'lua> FromLua<'lua> for LinkType {
        #[inline]
        fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
            match lua_value {
                LuaValue::String(s) => match s.to_str()? {
                    "Link" => Ok(Self::Link),
                    "Copy" => Ok(Self::Copy),
                    _ => conv_err(
                        LuaValue::String(s),
                        "LinkType",
                        r#"string ("Link" or "Copy")"#,
                    ),
                },
                _ => conv_err(lua_value, "LinkType", r#"string ("Link" or "Copy")"#),
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
