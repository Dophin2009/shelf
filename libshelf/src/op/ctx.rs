use std::collections::hash_map::DefaultHasher;
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Context object passed into [`super::Finish::finish`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FinishCtx {
    pub filesafe: FileSafe,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileSafe {
    path: PathBuf,
}

impl FinishCtx {
    #[inline]
    pub fn new(filesafe: FileSafe) -> Self {
        Self { filesafe }
    }
}

impl FileSafe {
    #[inline]
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[inline]
    pub fn resolve<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        let mut hasher = DefaultHasher::new();
        path.as_ref().hash(&mut hasher);
        let hash = hasher.finish();

        self.path.join(hash.to_string())
    }
}
