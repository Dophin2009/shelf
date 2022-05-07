use std::path::Path;

use shelflib::load::LoadError;

use crate::ctxpath::CtxPath;
use crate::output::{self, comb};

#[inline]
pub fn loading(path: &CtxPath) {
    output::section("loading", path.rel().display());
}

#[inline]
pub fn skip(_path: &CtxPath) {
    output::step("already done");
}

#[inline]
pub fn debug_reading() {
    output::step("reading package");
}

#[inline]
pub fn debug_evaling() {
    output::step("evaluating lua");
}

#[inline]
pub fn debug_queue_dep(dep: &CtxPath, parent: &Path) {
    let dep_rel = CtxPath::new(dep.abs(), &parent).unwrap();
    output::step(comb::sjoin2(
        "queueing dependency",
        output::path(dep_rel.rel()),
    ));
}

#[inline]
pub fn error_loading(errors: Vec<(CtxPath, LoadError)>) {
    output::section_error("encountered errors while trying to load packages");

    for (path, err) in errors.into_iter() {
        error_loading_path(path, err);
    }
}

#[inline]
pub fn error_loading_path(path: CtxPath, err: LoadError) {
    output::step_error_context(output::path(path.abs()));

    let message = match err {
        // TODO: More specific error messages
        LoadError::Read(_err) => comb::sjoin3(
            "couldn't read the package config; are you sure",
            output::path("package.lua"),
            "exists?",
        ),
        LoadError::Lua(err) => comb::sjoin2("couldn't evaluate Lua:", err),
    };

    output::step_error(message);
}
