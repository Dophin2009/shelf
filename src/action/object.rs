use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Object(pub HashMap<String, Value>);

// FIXME Custom serialization/deserialization to handle Nil?
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

impl Object {
    #[inline]
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}
