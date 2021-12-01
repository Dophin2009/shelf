pub use super::Tree;

use std::path::PathBuf;

use super::{Res, Resolve, ResolveOpts, WriteAction, WriteActionError};

#[derive(Debug, Clone)]
pub struct YamlAction {
    pub dest: PathBuf,
    pub values: Tree,

    pub header: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum YamlActionError {
    #[error("serde error")]
    Serde(#[from] serde_yaml::Error),
}

impl<'lua> Resolve<'lua> for YamlAction {
    type Error = YamlActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let Self {
            dest,
            values,
            header,
        } = self;

        // Render contents.
        let mut contents = serde_yaml::to_string(&values)?;
        if let Some(header) = header {
            contents.insert_str(0, header);
            contents.insert(0, '\n');
        }

        // Write contents.
        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        let resolution = WriteActionError::unwrap(wa.resolve(opts));
        Ok(resolution)
    }
}

#[derive(Debug, Clone)]
pub struct TomlAction {
    pub dest: PathBuf,
    pub values: Tree,

    pub header: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TomlActionError {
    #[error("serde error")]
    Serde(#[from] toml::ser::Error),
}

impl<'lua> Resolve<'lua> for TomlAction {
    type Error = TomlActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let Self {
            dest,
            values,
            header,
        } = self;

        // Render contents.
        let mut contents = toml::to_string_pretty(&values)?;
        if let Some(header) = header {
            contents.insert_str(0, header);
            contents.insert(0, '\n');
        }

        // Write contents.
        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        let resolution = WriteActionError::unwrap(wa.resolve(opts));
        Ok(resolution)
    }
}

#[derive(Debug, Clone)]
pub struct JsonAction {
    pub dest: PathBuf,
    pub values: Tree,
}

#[derive(Debug, thiserror::Error)]
pub enum JsonActionError {
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
}

impl<'lua> Resolve<'lua> for JsonAction {
    type Error = JsonActionError;

    #[inline]
    fn resolve(&self, opts: &ResolveOpts) -> Result<Res<'lua>, Self::Error> {
        let Self { dest, values } = self;

        // Render contents.
        let contents = serde_json::to_string(&values)?;

        // Write contents.
        let wa = WriteAction {
            dest: dest.clone(),
            contents,
        };
        let resolution = WriteActionError::unwrap(wa.resolve(opts));
        Ok(resolution)
    }
}
