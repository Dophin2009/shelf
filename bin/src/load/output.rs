use std::path::Path;

use shelflib::load::LoadError;

use crate::ctxpath::CtxPath;
use crate::output::{comb, spath, Section, Step};

#[inline]
pub fn loading(path: &CtxPath) {
    Section::message("loading", path.rel().display());
}

#[inline]
pub fn skip(_path: &CtxPath) {
    Step::message("already done");
}

#[inline]
pub fn reading() {
    Step::message("reading package");
}

#[inline]
pub fn evaling() {
    Step::message("evaluating lua");
}

#[inline]
pub fn queueing_dep(dep: &CtxPath, parent: &Path) {
    let dep_rel = CtxPath::new(dep.abs(), &parent).unwrap();
    Step::message(comb::sjoin2("queueing dependency", spath(dep_rel.rel())));
}

#[inline]
pub fn error_loading(errors: Vec<(CtxPath, LoadError)>) {
    Step::error().message("encountered errors while trying to load packages");

    for (path, err) in errors.into_iter() {
        error_loading_path(path, err);
    }
}

#[inline]
pub fn error_loading_path(path: CtxPath, err: LoadError) {
    Step::error().context(spath(path.abs()));

    let message = match err {
        // TODO: More specific error messages
        LoadError::Read(_err) => comb::sjoin3(
            "couldn't read the package config; are you sure",
            spath("package.lua"),
            "exists?",
        ),
        LoadError::Lua(err) => comb::sjoin2("couldn't evaluate Lua:", err),
    };

    Step::error().message(message);
}
