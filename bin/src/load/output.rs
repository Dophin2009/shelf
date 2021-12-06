use std::path::PathBuf;

use shelflib::load::LoadError;

use crate::ctxpath::CtxPath;
use crate::pretty::output::Emit;
use crate::pretty::semantic::{arrow, context, error, info, ppath};
use crate::pretty::{indent2, indent4, joins2, joins3, paren};

#[inline]
pub fn info_loading(path: &CtxPath) {
    info(joins2("Loading package", ppath(path.rel()))).info();
}

#[inline]
pub fn info_loading_skip(path: &CtxPath) {
    info(joins3(
        "Loading package",
        ppath(path.rel()),
        paren("already done"),
    ))
    .info();
}

#[inline]
pub fn debug_reading() {
    indent2(arrow("Reading package")).debug();
}

#[inline]
pub fn debug_evaling() {
    indent2(arrow("Evaluating Lua")).debug();
}

#[inline]
pub fn debug_queue_dep(dpath: &CtxPath, path: &PathBuf) {
    let dpath_rel = CtxPath::new(dpath.abs(), &path).unwrap();
    indent2(arrow(joins2("Queueing dependency", ppath(dpath_rel.rel())))).debug();
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
        LoadError::Read(err) => joins3(
            "couldn't read the package config; are you sure",
            ppath("package.lua"),
            "exists?",
        ),
        LoadError::Lua(err) => joins2("couldn't evaluate Lua:", err),
    };

    indent4(message).error();
}
