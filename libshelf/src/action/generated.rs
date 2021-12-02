use std::path::PathBuf;

use super::write::WriteAction;
use super::Resolve;

// Re-export action types.
pub use self::{json::JsonAction, toml::TomlAction, yaml::YamlAction};
// Re-export shared Res type.
pub use super::write::Res;
// Re-export shared Object type.
pub use crate::object::Object;

pub mod yaml {
    use std::path::PathBuf;

    use super::{Object, Res, Resolve};

    #[derive(Debug, Clone)]
    pub struct YamlAction {
        pub dest: PathBuf,
        pub values: Object,

        pub header: Option<String>,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("serde error")]
        Serde(#[from] serde_yaml::Error),
    }

    impl Resolve for YamlAction {
        type Output = Result<Res, Error>;

        #[inline]
        fn resolve(&self) -> Self::Output {
            let Self {
                dest,
                values,
                header,
            } = self;

            // Render contents.
            let mut contents = serde_yaml::to_string(&values)?;
            Ok(super::write_resolve(dest, contents, header))
        }
    }
}

pub mod toml {
    use std::path::PathBuf;

    use super::{Object, Res, Resolve};

    #[derive(Debug, Clone)]
    pub struct TomlAction {
        pub dest: PathBuf,
        pub values: Object,

        pub header: Option<String>,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("serde error")]
        Serde(#[from] toml::ser::Error),
    }

    impl Resolve for TomlAction {
        type Output = Result<Res, Error>;

        #[inline]
        fn resolve(&self) -> Self::Output {
            let Self {
                dest,
                values,
                header,
            } = self;

            // Render contents.
            let mut contents = toml::to_string_pretty(&values)?;
            Ok(super::write_resolve(dest, contents, header))
        }
    }
}

pub mod json {
    use std::path::PathBuf;

    use super::{Object, Res, Resolve};

    #[derive(Debug, Clone)]
    pub struct JsonAction {
        pub dest: PathBuf,
        pub values: Object,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("serde error")]
        Serde(#[from] serde_json::Error),
    }

    impl Resolve for JsonAction {
        type Output = Result<Res, Error>;

        #[inline]
        fn resolve(&self) -> Self::Output {
            let Self { dest, values } = self;

            // Render contents.
            let contents = serde_json::to_string(&values)?;
            Ok(super::write_resolve(dest, contents, &None));
        }
    }
}

#[inline]
fn write_resolve(dest: &PathBuf, contents: String, header: &Option<String>) -> Res {
    if let Some(header) = header.as_ref() {
        contents.insert_str(0, header);
        contents.insert(0, '\n');
    }

    // Write contents.
    let wa = WriteAction {
        dest: dest.clone(),
        contents: contents.into_bytes(),
    };

    wa.resolve()
}
