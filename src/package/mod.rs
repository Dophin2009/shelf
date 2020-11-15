mod config;
mod map;

pub use config::*;
pub use map::*;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    #[serde(default)]
    pub variables: Map,
    #[serde(flatten)]
    pub config: Config,
}

impl Package {
    pub fn new_optional(config: Config, variables: Option<Map>) -> Self {
        Self {
            config,
            variables: variables.unwrap_or_default(),
        }
    }
}
