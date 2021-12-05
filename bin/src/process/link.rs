use std::path::PathBuf;

use shelflib::action::{
    link::{Error, Op, Res, Skip},
    LinkAction, Resolve,
};

use crate::ctxpath::CtxPath;
use crate::pretty::{
    arrshowdim, error,
    output::{sl_debug, sl_info, sli_debug, sli_error, sli_warn, slii_error, slii_warn},
    ppath, skipping, var, warning,
};

#[inline]
pub fn process(action: LinkAction, path: &CtxPath, dest: &PathBuf) -> Result<(), ()> {
    let src = CtxPath::new(&action.src, path.abs()).unwrap();
    let dest = CtxPath::new(&action.dest, dest).unwrap();

    sl_info(format!(
        "{} {} to {}",
        if action.copy { "Copying" } else { "Linking" },
        ppath(src.rel()),
        ppath(dest.rel())
    ));
    sli_debug(arrshowdim(src.abs().display()));
    sli_debug(arrshowdim(dest.abs().display()));

    let res = match action.resolve() {
        Ok(res) => res,
        Err(err) => {
            process_error(&src, &dest, err);
            return Err(());
        }
    };

    match res {
        Res::Normal(ops) => process_normal(&src, &dest, ops),
        Res::Overwrite(ops) => process_overwrite(&src, &dest, ops),
        Res::Skip(skip) => process_skip(&src, &dest, skip),
    }

    Ok(())
}

#[inline]
fn process_normal(src: &CtxPath, dest: &CtxPath, ops: Vec<Op>) {
    // TODO: Implement
}

#[inline]
fn process_overwrite(src: &CtxPath, dest: &CtxPath, ops: Vec<Op>) {
    sli_warn(warning(format!(
        "{} will be overwritten",
        ppath(dest.rel())
    )));
    slii_warn(arrshowdim(dest.abs().display()));

    // TODO: Implement
}

#[inline]
fn process_skip(src: &CtxPath, dest: &CtxPath, skip: Skip) {
    match skip {
        Skip::SameSrcDest => {
            sli_warn(skipping(format!(
                "{} and {} are the same",
                var("src"),
                var("dest")
            )));
            slii_warn(arrshowdim(src.abs().display()));
            slii_warn(arrshowdim(dest.abs().display()));
        }
        Skip::OptMissing => {
            sli_warn(skipping(format!(
                "optional {} {} is missing",
                var("src"),
                ppath(src.rel())
            )));
            slii_warn(arrshowdim(src.abs().display()));
        }
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

#[inline]
fn process_error(src: &CtxPath, _dest: &CtxPath, err: Error) {
    match err {
        Error::SrcMissing => {
            sli_error(error(format!(
                "{} missing: {}",
                var("src"),
                ppath(src.rel())
            )));
            slii_error(arrshowdim(src.abs().display()));
        }
    }
}
