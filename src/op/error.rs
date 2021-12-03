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

/// Error encountered when creating a directory.
#[derive(Debug, thiserror::Error)]
#[error("i/o mkdir error")]
pub struct MkdirError {
    pub path: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when symlinking a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o create error")]
pub struct SymlinkError {
    pub src: PathBuf,
    pub dest: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when copying a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o copy error")]
pub struct CopyError {
    pub src: PathBuf,
    pub dest: PathBuf,
    #[source]
    pub inner: io::Error,
}

/// Error encountered when renaming a file.
#[derive(Debug, thiserror::Error)]
#[error("i/o rename error")]
pub struct RenameError {
    pub src: PathBuf,
    pub dest: PathBuf,
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

/// Error encountered when spawning a command.
#[derive(Debug, thiserror::Error)]
#[error("i/o command spawn error")]
pub struct SpawnError {
    pub command: String,
    pub shell: String,
    pub start: PathBuf,

    #[source]
    pub inner: io::Error,
}
