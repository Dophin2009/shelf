use std::fs;
use std::path::Path;

#[inline]
pub fn exists<P>(path: AsRef<Path>) -> bool {
    fs::symlink_metadata(path).is_ok()
}
