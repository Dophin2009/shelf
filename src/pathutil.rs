use std::env;
use std::path::{Path, PathBuf};

use path_clean::PathClean;

#[derive(Debug, Clone)]
pub struct PathWrapper {
    rel: Option<PathBuf>,
    abs: PathBuf,
}

impl PathWrapper {
    #[inline]
    pub fn new<P1, P2>(rel: P1, abs: P2) -> Self
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
    {
        Self::_new(Some(rel.into()), abs.into())
    }

    #[inline]
    fn _new<P1, P2>(rel: Option<P1>, abs: P2) -> Self
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
    {
        Self {
            rel: rel.map(Into::into),
            abs: abs.into(),
        }
        .clean()
    }

    #[inline]
    pub fn from_cwd<P>(path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let path: PathBuf = path.into();
        let (rel, abs) = if path.is_absolute() {
            (None, path)
        } else {
            let cwd = env::current_dir().unwrap();
            let abs = cwd.join(&path);
            (Some(path), abs)
        };

        Self::_new(rel, abs)
    }

    #[inline]
    pub fn from_with_start<P, S>(path: P, start: S) -> Self
    where
        P: Into<PathBuf>,
        S: AsRef<Path>,
    {
        let path: PathBuf = path.into();
        let (rel, abs) = if path.is_absolute() {
            (None, path)
        } else {
            let abs = start.as_ref().join(&path);
            (Some(path), abs)
        };
        Self::_new(rel, abs)
    }

    #[inline]
    pub fn rel(&self) -> &PathBuf {
        match self.rel.as_ref() {
            Some(rel) => rel,
            None => self.abs(),
        }
    }

    #[inline]
    pub fn abs(&self) -> &PathBuf {
        &self.abs
    }

    #[inline]
    pub fn reld(&self) -> std::path::Display<'_> {
        self.rel.as_ref().unwrap_or_else(|| &self.abs).display()
    }

    #[inline]
    pub fn absd(&self) -> std::path::Display<'_> {
        self.abs().display()
    }

    #[inline]
    pub fn exists(&self) -> bool {
        self.abs().exists()
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.abs().is_dir()
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        self.abs().is_file()
    }

    #[inline]
    pub fn parent(&self) -> Option<PathWrapper> {
        self.abs().parent().map(|abs| {
            let rel = self
                .rel()
                .parent()
                .map(|parent| parent.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".."));
            PathWrapper::new(rel, abs.to_path_buf())
        })
    }

    #[inline]
    pub fn join<P>(&self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            rel: self.rel.as_ref().map(|rel| rel.join(&path)),
            abs: self.abs.join(&path),
        }
    }

    #[inline]
    pub fn clean(&self) -> Self {
        Self {
            rel: self.rel.as_ref().map(|rel| rel.clean()),
            abs: self.abs.clean(),
        }
    }
}
