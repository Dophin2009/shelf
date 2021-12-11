use shelflib::graph::CircularDependencyError;

use crate::ctxpath::CtxPath;
use crate::pretty::{
    indent2, joins2,
    output::Emit,
    pretty,
    semantic::{contextm, error, },
    Prettify, Pretty,
};

#[inline]
fn processing() -> Pretty<&'static str> {
    pretty("processing").dim().bold()
}

#[inline]
pub fn info_processing(path: &CtxPath) {
    joins2(processing(), path.rel().display()).info();
}

#[inline]
pub fn error_circular_dep(err: CircularDependencyError) {
    error("circular dependency detected").error();
    indent2(contextm("in", err.path().display())).error();
}
