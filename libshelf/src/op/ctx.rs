use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Context object passed into [`super::Finish::finish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FinishCtx {
    pub file_safe: FileSafe,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileSafe {
    path: PathBuf,
}

impl FinishCtx {
    #[inline]
    pub fn new(file_safe: FileSafe) -> Self {
        Self { file_safe }
    }
}

impl FileSafe {
    #[inline]
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self { path: path.as_ref().to_path_buf() }
    }

    #[inline]
    pub fn resolve<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.path.join(path)
    }
}
