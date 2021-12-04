use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::{env, io};

use shelflib::fse;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CtxPath {
    rel: PathBuf,
    abs: PathBuf,
}

impl CtxPath {
    #[inline]
    pub fn new<P, S>(path: P, start: S) -> Option<Self>
    where
        P: Into<PathBuf>,
        S: AsRef<Path>,
    {
        let path = fse::clean(path.into());
        let start = fse::clean(start);

        let (rel, abs) = match (path.is_absolute(), start.is_absolute()) {
            // Get relative by taking diff of absolute and start.
            (true, true) => {
                // SAFETY: `path` is absolute.
                let rel = pathdiff::diff_paths(&path, start).unwrap();
                (rel, path)
            }
            // Append path to start to get absolute.
            (false, true) => {
                let abs = fse::clean(start.join(&path));

                // SAFETY: `abs` is absolute.
                let rel = pathdiff::diff_paths(&abs, start).unwrap();
                (rel, abs)
            }
            // Cannot get relative from a relative start.
            (_, false) => return None,
        };

        Some(Self { rel, abs }.cleaned())
    }

    #[inline]
    pub fn from_cwd<P>(path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        // SAFETY: `cwd` should be absolute?
        let cwd = env::current_dir().unwrap();
        Self::new(path, cwd).unwrap()
    }

    #[inline]
    pub fn rel(&self) -> &Path {
        &self.rel
    }

    #[inline]
    pub fn abs(&self) -> &Path {
        &self.abs
    }

    #[inline]
    pub fn metadata(&self) -> io::Result<Metadata> {
        self.abs().metadata()
    }

    #[inline]
    pub fn symlink_metadata(&self) -> io::Result<Metadata> {
        self.abs().symlink_metadata()
    }

    #[inline]
    pub fn exists(&self) -> bool {
        self.abs().exists()
    }

    #[inline]
    pub fn symlink_exists(&self) -> bool {
        fse::symlink_exists(self.abs())
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
    pub fn is_symlink(&self) -> bool {
        self.abs().is_symlink()
    }

    #[inline]
    pub fn join<P>(&self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            rel: self.rel.join(&path),
            abs: self.abs.join(path),
        }
    }

    #[inline]
    pub fn cleaned(self) -> Self {
        Self {
            rel: fse::clean(self.rel),
            abs: fse::clean(self.abs),
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::CtxPath;

    #[test]
    fn test_new_rel() {
        let a = CtxPath::new("a", "/").unwrap();
        assert_eq!(a.abs(), &PathBuf::from("/a"));
        assert_eq!(a.rel(), &PathBuf::from("a"));

        let b = CtxPath::new("b", "/a").unwrap();
        assert_eq!(b.abs(), &PathBuf::from("/a/b"));
        assert_eq!(b.rel(), &PathBuf::from("b"));

        let c = CtxPath::new("../c", "/a").unwrap();
        assert_eq!(c.abs(), &PathBuf::from("/c"));
        assert_eq!(c.rel(), &PathBuf::from("../c"));

        let d = CtxPath::new("../../d", "/a/b/c").unwrap();
        assert_eq!(d.abs(), &PathBuf::from("/a/d"));
        assert_eq!(d.rel(), &PathBuf::from("../../d"));

        let e = CtxPath::new("../c/d", "/a/b/c").unwrap();
        assert_eq!(e.abs(), &PathBuf::from("/a/b/c/d"));
        assert_eq!(e.rel(), &PathBuf::from("d"));
    }

    #[test]
    fn test_new_abs() {
        let a = CtxPath::new("/a", "/").unwrap();
        assert_eq!(a.abs(), &PathBuf::from("/a"));
        assert_eq!(a.rel(), &PathBuf::from("a"));

        let b = CtxPath::new("/a/b", "/a").unwrap();
        assert_eq!(b.abs(), &PathBuf::from("/a/b"));
        assert_eq!(b.rel(), &PathBuf::from("b"));

        let c = CtxPath::new("/c", "/a").unwrap();
        assert_eq!(c.abs(), &PathBuf::from("/c"));
        assert_eq!(c.rel(), &PathBuf::from("../c"));

        let d = CtxPath::new("/a/d", "/a/b/c").unwrap();
        assert_eq!(d.abs(), &PathBuf::from("/a/d"));
        assert_eq!(d.rel(), &PathBuf::from("../../d"));

        let e = CtxPath::new("/a/d", "/a/b/c").unwrap();
        assert_eq!(e.abs(), &PathBuf::from("/a/d"));
        assert_eq!(e.rel(), &PathBuf::from("../../d"));
    }

    #[test]
    fn test_new_none() {
        let a = CtxPath::new("a", "a");
        assert_eq!(a, None);

        let b = CtxPath::new("b", "");
        assert_eq!(b, None);

        let c = CtxPath::new("c", "a/b");
        assert_eq!(c, None);
    }
}
