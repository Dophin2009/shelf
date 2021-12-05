use std::path::PathBuf;

use shelflib::action::link::{Res, Skip};
use shelflib::action::{link::Error, LinkAction, Resolve};

use crate::ctxpath::CtxPath;
use crate::pretty::error;
use crate::pretty::{
    arrshowdim,
    output::{sl_debug, sli_error, sli_warn, slii_error, slii_warn},
    ppath, skipping, var,
};

#[inline]
pub fn process(action: LinkAction, path: &CtxPath, dest: &PathBuf) -> Result<(), ()> {
    let src = CtxPath::new(&action.src, path.abs()).unwrap();
    let dest = CtxPath::new(&action.dest, dest).unwrap();

    sl_debug(format!(
        "{} {} to {}",
        if action.copy { "Copying" } else { "Linking" },
        ppath(src.rel()),
        ppath(dest.rel())
    ));

    let res = match action.resolve() {
        Ok(res) => res,
        Err(err) => {
            process_error(&src, &dest, err);
            return Err(());
        }
    };

    match res {
        Res::Normal(ops) => todo!(),
        Res::Overwrite(ops) => todo!(),
        Res::Skip(skip) => process_skip(&src, &dest, skip),
    }

    Ok(())
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
        Skip::OptMissing => {}
        Skip::DestExists => {}
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
