use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub trait Cache {
    fn set_file(&mut self, package: PathBuf, path: PathBuf, fm: FileMeta) -> Option<FileMeta>;
    fn get_file(&self, package: PathBuf, path: PathBuf, fm: FileMeta) -> Option<&FileMeta>;
}

#[derive(Debug, Clone)]
pub struct FsCache {
    path: PathBuf,

    packages: HashMap<PathBuf, PackageMeta>,
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
            packages: HashMap::new(),
        }
    }

    #[inline]
    pub fn read_from(path: impl AsRef<Path>) -> Result<Self, FsCacheError> {
        let file = Self::open_file(&path)?;
        let rdr = BufReader::new(file);
        let packages = serde_json::from_reader(rdr)?;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            packages,
        })
    }

    #[inline]
    pub fn write(&self) -> Result<(), FsCacheError> {
        let file = Self::open_file(&self.path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.packages)?;

        Ok(())
    }

    #[inline]
    fn open_file<P: AsRef<Path>>(path: P) -> Result<File, FsCacheError> {
        let file = File::open(path)?;
        Ok(file)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.packages.clear();
    }
}

impl Cache for FsCache {
    #[inline]
    fn set_file(&mut self, package: PathBuf, path: PathBuf, fm: FileMeta) -> Option<FileMeta> {
        self.packages
            .get_mut(&package)
            .map(|pm| pm.set(path, fm))
            .flatten()
    }

    #[inline]
    fn get_file(&self, package: PathBuf, path: PathBuf, fm: FileMeta) -> Option<&FileMeta> {
        self.packages
            .get(&package)
            .map(|pm| pm.get(&path))
            .flatten()
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
    fn set_file(&mut self, _package: PathBuf, _path: PathBuf, _fm: FileMeta) -> Option<FileMeta> {
        None
    }

    #[inline]
    fn get_file(&self, _package: PathBuf, _path: PathBuf, _fm: FileMeta) -> Option<&FileMeta> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    files: HashMap<PathBuf, FileMeta>,
}

impl PackageMeta {
    #[inline]
    pub fn set(&mut self, path: impl AsRef<Path>, fm: FileMeta) -> Option<FileMeta> {
        self.files.insert(path.as_ref().to_path_buf(), fm)
    }

    #[inline]
    pub fn get(&self, path: impl AsRef<Path>) -> Option<&FileMeta> {
        self.files.get(path.as_ref())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileMeta {
    pub typ: FileTyp,
}

impl FileMeta {
    #[inline]
    pub fn new(typ: FileTyp) -> Self {
        Self { typ }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FileTyp {}
