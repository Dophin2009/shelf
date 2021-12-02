use std::fs;
use std::path::Path;

#[inline]
pub fn symlink_exists<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    fs::symlink_metadata(path).is_ok()
}
