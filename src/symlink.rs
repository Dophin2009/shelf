use std::io;
use std::path::Path;

#[cfg(windows)]
pub fn symlink<P: AsRef<Path>>(src: P, dest: P) -> io::Result<()> {
    use std::os::windows;
    windows::fs::symlink_file(src, dest)
}

#[cfg(unix)]
pub fn symlink<P: AsRef<Path>>(src: P, dest: P) -> io::Result<()> {
    use std::os::unix;
    unix::fs::symlink(src, dest)
}
