use std::path::PathBuf;

use shelflib::action::{
    write::{Op, Res, Skip},
    Resolve, WriteAction,
};

use crate::ctxpath::CtxPath;
use crate::pretty::{
    output::{sl_info, sli_debug, sli_warn, slii_warn},
    semantic::{arrshowdim, ppath, skipping, var, warning},
};

#[inline]
pub fn process(action: WriteAction, path: &CtxPath, dest: &PathBuf) -> Result<(), ()> {
    let dest = CtxPath::new(&action.dest, dest).unwrap();

    sl_info(format!("Writing {}", ppath(dest.rel())));
    sli_debug(arrshowdim(dest.abs().display()));

    let res = action.resolve();

    match res {
        Res::Normal(ops) => process_normal(&dest, ops),
        Res::OverwriteContents(ops) => process_overwrite_contents(&dest, ops),
        Res::OverwriteFile(ops) => process_overwrite_file(&dest, ops),
        Res::Skip(skip) => process_skip(&dest, skip),
    }

    Ok(())
}

#[inline]
fn process_normal(dest: &CtxPath, ops: Vec<Op>) {
    // TODO: Implement
}

#[inline]
fn process_overwrite_contents(dest: &CtxPath, ops: Vec<Op>) {
    sli_warn(warning(format!(
        "{} will be overwritten",
        ppath(dest.rel())
    )));
    slii_warn(arrshowdim(dest.abs().display()));

    // TODO: Implement
}

#[inline]
fn process_overwrite_file(dest: &CtxPath, ops: Vec<Op>) {
    sli_warn(warning(format!("{} will be replaced", ppath(dest.rel()))));
    slii_warn(arrshowdim(dest.abs().display()));

    // TODO: Implement
}

#[inline]
fn process_skip(dest: &CtxPath, skip: Skip) {
    match skip {
        Skip::DestExists => {
            sli_warn(skipping(format!(
                "{} {} already exists",
                var("dest"),
                ppath(dest.rel())
            )));
            slii_warn(arrshowdim(dest.abs().display()));
        }
    }
}
