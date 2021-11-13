use std::io;
use std::path::PathBuf;

/// Error encountered when opening a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o open error")]
pub struct OpenError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when creating a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o create error")]
pub struct CreateError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when reading a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o read error")]
pub struct ReadError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when writing a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o read error")]
pub struct WriteError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when removing a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o remove error")]
pub struct RemoveError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}
