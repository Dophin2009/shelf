use std::collections::HashMap;

use gtmpl::Value as GtmplValue;
use gtmpl_derive::Gtmpl;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Gtmpl)]
pub struct Map {
    #[serde(default, flatten)]
    pub map: HashMap<String, Value>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Object(HashMap<String, Value>),
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
}

impl Into<GtmplValue> for Value {
    fn into(self) -> GtmplValue {
        match self {
            Value::Object(map) => map.into(),
            Value::Integer(n) => n.into(),
            Value::Float(f) => f.into(),
            Value::String(s) => s.into(),
            Value::Bool(b) => b.into(),
            Value::Nil => GtmplValue::Nil,
        }
    }
}
