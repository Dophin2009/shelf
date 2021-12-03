use std::fs;
use std::path::{Component, Path, PathBuf};

#[inline]
pub fn symlink_exists<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    fs::symlink_metadata(path).is_ok()
}

#[inline]
pub fn clean<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    if path.as_ref().components().count() <= 1 {
        return path.as_ref().to_owned();
    }

    let mut components = Vec::new();

    for component in path
        .as_ref()
        .components()
        .filter(|component| component != &Component::CurDir)
    {
        if component == Component::ParentDir {
            match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                Some(Component::ParentDir) | None => components.push(component),
                _ => {}
            }
        } else {
            components.push(component);
        }
    }

    components.into_iter().collect()
}
