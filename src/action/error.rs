use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
#[error("file is missing")]
pub struct FileMissingError {
    pub path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("cannot read file metadata")]
pub struct FileReadMetadataError {
    pub path: PathBuf,
    #[source]
    pub err: io::Error,
}
