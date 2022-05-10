use shelflib::graph::CircularDependencyError;

use crate::ctxpath::CtxPath;
use crate::output::Section;

#[inline]
pub fn processing(path: &CtxPath) {
    Section::message("processing", path.rel().display());
}

#[inline]
pub fn error_circular(err: CircularDependencyError) {
    Section::error().message("circular dependency detected");
    Section::error().context(err.path().display());
}
