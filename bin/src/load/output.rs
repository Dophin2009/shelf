use std::path::Path;

use shelflib::load::LoadError;

use crate::ctxpath::CtxPath;
use crate::pretty::{
    indent2, indent4, joins2, joins3,
    output::Emit,
    pretty,
    semantic::{arrow, context, error, ppath},
    Prettify, Pretty,
};

#[inline]
fn loading() -> Pretty<&'static str> {
    pretty("loading").dim().bold()
}

#[inline]
pub fn info_loading(path: &CtxPath) {
    joins2(loading(), path.rel().display()).info();
}

#[inline]
pub fn debug_skip(_path: &CtxPath) {
    // TODO: Print path?
    indent2(arrow("already done")).debug();
}

#[inline]
pub fn debug_reading() {
    indent2(arrow("reading package")).debug();
}

#[inline]
pub fn debug_evaling() {
    indent2(arrow("evaluating Lua")).debug();
}

#[inline]
pub fn debug_queue_dep(dpath: &CtxPath, path: &Path) {
    let dpath_rel = CtxPath::new(dpath.abs(), &path).unwrap();
    indent2(arrow(joins2("queueing dependency", ppath(dpath_rel.rel())))).debug();
}

#[inline]
pub fn error_loading() {
    error("encountered errors while trying to load packages").error();
}

#[inline]
pub fn error_loading_path(path: &CtxPath, err: LoadError) {
    indent2(context(joins2("in", ppath(path.abs())))).error();

    let message = match err {
        // TODO: More specific error messages
        LoadError::Read(_err) => joins3(
            "couldn't read the package config; are you sure",
            ppath("package.lua"),
            "exists?",
        ),
        LoadError::Lua(err) => joins2("couldn't evaluate Lua:", err),
    };

    indent4(message).error();
}
