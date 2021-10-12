use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
pub struct NoError {}

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
