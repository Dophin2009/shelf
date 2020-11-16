use crate::package::{Map, MapValue};

use anyhow::{anyhow, Result};
use gtmpl::Value as GtmplValue;

pub fn render<T: Into<gtmpl::Value>>(template: &str, context: T) -> Result<String> {
    let rendered = gtmpl::template(template, context)
        .map_err(|err| anyhow!("Failed to render template: {}", err))?;
    Ok(rendered)
}

impl Into<GtmplValue> for Map {
    fn into(self) -> GtmplValue {
        self.map.into()
    }
}

impl Into<GtmplValue> for MapValue {
    fn into(self) -> GtmplValue {
        match self {
            MapValue::Object(map) => map.into(),
            MapValue::Integer(n) => n.into(),
            MapValue::Float(f) => f.into(),
            MapValue::String(s) => s.into(),
            MapValue::Bool(b) => b.into(),
            MapValue::Nil => GtmplValue::Nil,
        }
    }
}
