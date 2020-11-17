use super::{
    File, Hook, HookBody, LinkType, Map, MapValue, Package, PackageFiles, PackageHooks, Template,
    TemplateType, Tree,
};

use std::collections::HashMap;

use mlua::{Error as LuaError, FromLua, Function, Lua, Result as LuaResult, Value as LuaValue};
use uuid::Uuid;

macro_rules! t_get {
    ($table:ident, $key:expr, $lua:ident) => {
        FromLua::from_lua($table.get($key)?, $lua)?
    };
    ($table:ident, $lua:ident) => {
        FromLua::from_lua(LuaValue::Table($table), $lua)?
    };
}

impl<'lua> FromLua<'lua> for Package {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => Ok(Self {
                name: t_get!(t, "name", lua),
                dependencies: t_get!(t, "dependencies", lua),
                files: t_get!(t, "files", lua),
                hooks: t_get!(t, "hooks", lua),
                variables: t_get!(t, "variables", lua),
            }),
            _ => conv_err(lua_value, "Package", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for PackageFiles {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => Ok(Self {
                trees: t_get!(t, "trees", lua),
                extra: t_get!(t, "extra", lua),
                templates: t_get!(t, "templates", lua),
                link_type: t_get!(t, "link_type", lua),
                ignore: t_get!(t, "ignore", lua),
                replace_files: t_get!(t, "replace_files", lua),
                replace_dirs: t_get!(t, "replace_dirs", lua),
            }),
            _ => conv_err(lua_value, "PackageFiles", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for PackageHooks {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => Ok(Self {
                pre: t_get!(t, "pre", lua),
                install: t_get!(t, "install", lua),
                post: t_get!(t, "pre", lua),
            }),
            _ => conv_err(lua_value, "PackageHooks", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for Tree {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let path: String = t_get!(t, "path", lua);
                Ok(Self {
                    path: path.into(),
                    link_type: t_get!(t, "link_type", lua),
                    ignore: t_get!(t, "ignore", lua),
                    replace_files: t_get!(t, "replace_files", lua),
                    replace_dirs: t_get!(t, "replace_dirs", lua),
                })
            }
            _ => conv_err(lua_value, "Tree", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for File {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let src: String = t_get!(t, "src", lua);
                let dest: String = t_get!(t, "dest", lua);
                Ok(Self {
                    src: src.into(),
                    dest: dest.into(),
                    link_type: t_get!(t, "link_type", lua),
                    replace_files: t_get!(t, "replace_files", lua),
                    replace_dirs: t_get!(t, "replace_dirs", lua),
                })
            }
            _ => conv_err(lua_value, "File", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for LinkType {
    fn from_lua(lua_value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::String(s) => match s.to_str()?.to_lowercase().as_str() {
                "link" => Ok(Self::Link),
                "copy" => Ok(Self::Copy),
                _ => conv_err(LuaValue::String(s), "LinkType", "string ('link' or 'copy')"),
            },
            _ => conv_err(lua_value, "LinkType", "string ('link' or 'copy')"),
        }
    }
}

impl<'lua> FromLua<'lua> for Template {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let src: String = t_get!(t, "src", lua);
                let dest: String = t_get!(t, "dest", lua);
                let replace_files = t_get!(t, "replace_files", lua);
                let replace_dirs = t_get!(t, "replace_dirs", lua);
                Ok(Self {
                    src: src.into(),
                    dest: dest.into(),
                    ty: t_get!(t, lua),
                    replace_files,
                    replace_dirs,
                })
            }
            _ => conv_err(lua_value, "Template", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for TemplateType {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let err_message =
            Some("invalid template type, should be 'handlebars', 'gotmpl', or 'tera'".to_string());
        match lua_value {
            LuaValue::Table(t) => {
                let ty: String = t_get!(t, "type", lua);
                match ty.to_lowercase().as_str() {
                    "handlebars" => {
                        let partials: HashMap<String, String> = t_get!(t, "partials", lua);
                        Ok(Self::Handlebars {
                            partials: partials.into_iter().map(|(k, v)| (k, v.into())).collect(),
                        })
                    }
                    "gotmpl" => Ok(Self::Gotmpl),
                    "tera" => Ok(Self::Tera),
                    _ => Err(LuaError::FromLuaConversionError {
                        from: "string",
                        to: "TemplateType",
                        message: err_message,
                    }),
                }
            }
            _ => Err(LuaError::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "TemplateType",
                message: err_message,
            }),
        }
    }
}

impl<'lua> FromLua<'lua> for Hook {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let name = t_get!(t, "name", lua);
                let body = if t.contains_key("path")? {
                    let path: String = t_get!(t, "path", lua);
                    HookBody::Executable { path: path.into() }
                } else if t.contains_key("fn")? {
                    let func: Function<'lua> = t_get!(t, "fn", lua);
                    let name: String = Uuid::new_v4().to_string();
                    lua.set_named_registry_value(&name, func)?;
                    HookBody::LuaFunction { name }
                } else {
                    return Err(LuaError::FromLuaConversionError {
                        from: "nil",
                        to: "HookBody",
                        message: Some("neither 'path' nor 'fn' found".to_string()),
                    });
                };

                Ok(Self { name, body })
            }
            _ => conv_err(lua_value, "Hook", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for HookBody {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::String(s) => Ok(HookBody::Executable {
                path: s.to_str()?.into(),
            }),
            LuaValue::Function(func) => {
                let name: String = Uuid::new_v4().to_string();
                lua.set_named_registry_value(&name, func)?;
                Ok(HookBody::LuaFunction { name })
            }
            _ => conv_err(lua_value, "HookBody", "string or function"),
        }
    }
}

impl<'lua> FromLua<'lua> for Map {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => {
                let map: HashMap<String, MapValue> = t_get!(t, lua);
                Ok(Map { map })
            }
            _ => conv_err(lua_value, "Map", "table"),
        }
    }
}

impl<'lua> FromLua<'lua> for MapValue {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match lua_value {
            LuaValue::Table(t) => Ok(MapValue::Object(t_get!(t, lua))),
            LuaValue::Boolean(b) => Ok(MapValue::Bool(b)),
            LuaValue::Integer(n) => Ok(MapValue::Integer(n)),
            LuaValue::Number(f) => Ok(MapValue::Float(f)),
            LuaValue::String(s) => Ok(MapValue::String(s.to_str()?.to_string())),
            LuaValue::Nil => Ok(MapValue::Nil),
            _ => conv_err(
                lua_value,
                "Value",
                "table, bool, int, number, string, or nil",
            ),
        }
    }
}

fn conv_err<R>(value: LuaValue, to: &'static str, should: &str) -> LuaResult<R> {
    Err(LuaError::FromLuaConversionError {
        from: value.type_name(),
        to,
        message: Some(format!("must be a {}", should)),
    })
}
