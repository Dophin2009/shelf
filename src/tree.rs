use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tree(pub HashMap<String, Value>);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Object(HashMap<String, Value>),
}

impl Tree {
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

mod lua {
    use mlua::{Error as LuaError, FromLua, Value as LuaValue};

    use super::{Tree, Value};

    impl<'lua> FromLua<'lua> for Value {
        #[inline]
        fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
            let res = match lua_value {
                LuaValue::Nil => Value::Nil,
                LuaValue::Boolean(b) => Value::Bool(b),
                LuaValue::Integer(i) => Value::Int(i),
                LuaValue::Number(n) => Value::Float(n),
                LuaValue::String(s) => Value::Str(s.to_str()?.to_string()),
                LuaValue::Table(t) => Value::Object(FromLua::from_lua(LuaValue::Table(t), lua)?),
                LuaValue::Function(_)
                | LuaValue::Thread(_)
                | LuaValue::LightUserData(_)
                | LuaValue::UserData(_)
                | LuaValue::Error(_) => {
                    return Err(LuaError::FromLuaConversionError {
                        from: lua_value.type_name(),
                        to: "Value",
                        message: Some(
                            "Only nil, bool, int, float, string, and table values are valid"
                                .to_string(),
                        ),
                    })
                }
            };
            Ok(res)
        }
    }

    impl<'lua> FromLua<'lua> for Tree {
        #[inline]
        fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
            match lua_value {
                LuaValue::Table(t) => Ok(Tree(FromLua::from_lua(LuaValue::Table(t), lua)?)),
                _ => Err(LuaError::FromLuaConversionError {
                    from: lua_value.type_name(),
                    to: "Tree",
                    message: Some("Only table values are valid".to_string()),
                }),
            }
        }
    }
}
