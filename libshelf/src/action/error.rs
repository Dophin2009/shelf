use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
pub struct NoError {}

impl NoError {
    // Wish we had ! type :(
    #[inline]
    fn unwrap<T>(res: Result<T, Self>) -> T {
        res.unwrap_or_else(|| unreachable!())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub struct FileMissingError {
    pub path: PathBuf,
}

#[derive(Debug, Clone, thiserror::Error)]
pub struct FileReadMetadataError {
    pub path: PathBuf,
    #[source]
    pub err: io::Error,
}
