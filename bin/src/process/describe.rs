use std::path::Path;

use crate::ctxpath::CtxPath;
use crate::output::{self, Pretty};

#[derive(Debug, Clone, Copy)]
pub enum DescribeMode {
    Info,
    Error,
}

pub trait Describe {
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty;

    #[inline]
    fn describe_info(&self, path: &CtxPath, dest: &Path) -> Pretty {
        self.describe(path, dest, DescribeMode::Info)
    }

    #[inline]
    fn describe_error(&self, path: &CtxPath, dest: &Path) -> Pretty {
        self.describe(path, dest, DescribeMode::Error)
    }
}

#[inline]
pub fn path_relative(this: &Path, path: &CtxPath) -> CtxPath {
    CtxPath::new(this, path.abs()).unwrap()
}

#[inline]
pub fn spath_relative(this: &Path, path: &CtxPath) -> Pretty {
    output::spath(path_relative(this, path).rel())
}

#[inline]
pub fn dest_relative(this: &Path, dest: &Path) -> CtxPath {
    CtxPath::new(this, dest).unwrap()
}

#[inline]
pub fn sdest_relative(this: &Path, path: &Path) -> Pretty {
    output::spath(dest_relative(this, path).rel())
}

#[inline]
pub fn mode_spath(path: CtxPath, mode: DescribeMode) -> Pretty {
    let path = match mode {
        DescribeMode::Info => path.rel(),
        DescribeMode::Error => path.abs(),
    };
    output::spath(path)
}
