use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub trait Cache {
    type Error;

    fn insert(&mut self, path: PathBuf, fm: FileMeta) -> Option<FileMeta>;
    fn get(&self, path: &PathBuf) -> Option<&FileMeta>;
    fn get_mut(&mut self, path: &PathBuf) -> Option<&mut FileMeta>;
    fn remove(&mut self, path: &PathBuf) -> Option<FileMeta>;

    fn flush(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone)]
pub struct FsCache {
    path: PathBuf,

    files: HashMap<PathBuf, FileMeta>,
}

#[derive(Debug, thiserror::Error)]
pub enum FsCacheError {
    #[error("couldn't read cache file")]
    Read(#[from] io::Error),
    #[error("couldn't serialize/deserialize cache")]
    Serde(#[from] serde_json::Error),
}

impl FsCache {
    #[inline]
    pub fn empty(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            files: HashMap::new(),
        }
    }

    #[inline]
    pub fn read_from(path: impl AsRef<Path>) -> Result<Self, FsCacheError> {
        let file = Self::open_file(&path)?;
        let rdr = BufReader::new(file);
        let files = serde_json::from_reader(rdr)?;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            files,
        })
    }

    #[inline]
    pub fn write(&self) -> Result<(), FsCacheError> {
        let file = Self::open_file(&self.path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.files)?;

        Ok(())
    }

    #[inline]
    fn open_file<P: AsRef<Path>>(path: P) -> Result<File, FsCacheError> {
        let file = File::open(path)?;
        Ok(file)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.files.clear();
    }
}

impl Cache for FsCache {
    type Error = FsCacheError;

    #[inline]
    fn insert(&mut self, path: PathBuf, fm: FileMeta) -> Option<FileMeta> {
        self.files.insert(path, fm)
    }

    #[inline]
    fn get(&self, path: &PathBuf) -> Option<&FileMeta> {
        self.files.get(&path)
    }

    #[inline]
    fn get_mut(&mut self, path: &PathBuf) -> Option<&mut FileMeta> {
        self.files.get_mut(&path)
    }

    #[inline]
    fn remove(&mut self, path: &PathBuf) -> Option<FileMeta> {
        self.files.remove(path)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.write()
    }
}

#[derive(Debug, Clone)]
pub struct DummyCache;

impl DummyCache {
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Cache for DummyCache {
    #[inline]
    fn insert(&mut self, _path: PathBuf, _fm: FileMeta) -> Option<FileMeta> {
        None
    }

    #[inline]
    fn get(&self, _path: &PathBuf) -> Option<&FileMeta> {
        None
    }

    #[inline]
    fn get_mut(&mut self, _path: &PathBuf) -> Option<&mut FileMeta> {
        None
    }

    #[inline]
    fn remove(&mut self, _path: &PathBuf) -> Option<FileMeta> {
        None
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileMeta {
    pub typ: FileMetaTyp,
}

impl FileMeta {
    #[inline]
    pub fn new(typ: FileMetaTyp) -> Self {
        Self { typ }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FileMetaTyp {
    File { content_hash: u64 },
    Dir,
    Link,
}
