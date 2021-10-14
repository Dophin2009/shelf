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

fn conv_err<R>(value: LuaValue<'_>, to: &'static str, should: &str) -> mlua::Result<R> {
    Err(LuaError::FromLuaConversionError {
        from: value.type_name(),
        to,
        message: Some(format!("must be a {}", should)),
    })
}
