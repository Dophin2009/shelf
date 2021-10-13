use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
#[error("no error placeholder")]
pub struct NoError {}

impl NoError {
    // Wish we had ! type :(
    #[inline]
    fn unwrap<T>(res: Result<T, Self>) -> T {
        res.unwrap_or_else(|| unreachable!())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("file is missing")]
pub struct FileMissingError {
    pub path: PathBuf,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("cannot read file metadata")]
pub struct FileReadMetadataError {
    pub path: PathBuf,
    #[source]
    pub err: io::Error,
}
