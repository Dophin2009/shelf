use std::path::PathBuf;

use shelflib::action::{
    link::{Error, Op, Res, Skip},
    LinkAction, Resolve,
};

use crate::ctxpath::CtxPath;

#[inline]
pub fn process(action: LinkAction, path: &CtxPath, dest: &PathBuf) -> Result<(), ()> {
    let src = CtxPath::new(&action.src, path.abs()).unwrap();
    let dest = CtxPath::new(&action.dest, dest).unwrap();

    output::debug_process(action.copy, &src, &dest);
    let res = match action.resolve() {
        Ok(res) => res,
        Err(err) => {
            output::error_resolving(action.copy, &src, &dest, err);
            return Err(());
        }
    };

    match res {
        Res::Normal(ops) => process_normal(&src, &dest, ops),
        Res::Overwrite(ops) => {
            output::warn_overwrite(action.copy, &src, &dest);
            process_overwrite(&src, &dest, ops)
        }
        Res::Skip(skip) => {
            output::warn_skipping(action.copy, &src, &dest, skip);
            Ok(())
        }
    }
}

#[inline]
fn process_normal(_src: &CtxPath, _dest: &CtxPath, _ops: Vec<Op>) -> Result<(), ()> {
    // TODO: Implement

    Ok(())
}

#[inline]
fn process_overwrite(_src: &CtxPath, _dest: &CtxPath, _ops: Vec<Op>) -> Result<(), ()> {
    // TODO: Implement

    Ok(())
}

mod output {
    use shelflib::action::link::{Error, Skip};

    use crate::ctxpath::CtxPath;
    use crate::pretty::semantic::{skipping, warning};
    use crate::pretty::{
        indent2, indent4, indent6, joins2, joins3, joins4,
        output::Emit,
        semantic::{arrow, arrowdim, bulletdim, contextm, error, ppath, var},
        Pretty,
    };

    #[inline]
    pub fn debug_process(copy: bool, src: &CtxPath, dest: &CtxPath) {
        indent2(arrow(joins4(
            if copy { "copying" } else { "linking" },
            ppath(src.rel()),
            "to",
            ppath(dest.rel()),
        )))
        .debug();

        indent6(bulletdim(src.abs().display())).trace();
        indent6(bulletdim(dest.abs().display())).trace();
    }

    #[inline]
    pub fn warn_overwrite(copy: bool, src: &CtxPath, dest: &CtxPath) {
        indent2(warning(joins4(
            if copy { "copying" } else { "linking" },
            ppath(src.rel()),
            "to",
            ppath(dest.rel()),
        )))
        .warn();

        indent4(joins2(var("dest[2]"), "will be replaced")).warn();

        indent6(bulletdim(src.abs().display())).warn();
        indent6(bulletdim(dest.abs().display())).warn();
    }

    #[inline]
    pub fn warn_skipping(copy: bool, src: &CtxPath, dest: &CtxPath, skip: Skip) {
        indent2(skipping(joins4(
            if copy { "copying" } else { "linking" },
            ppath(src.rel()),
            "to",
            ppath(dest.rel()),
        )))
        .warn();

        match skip {
            Skip::SameSrcDest => {
                indent4(joins4(var("src[1]"), "and", var("dest[2]"), "are the same")).warn();
            }
            Skip::OptMissing => {
                indent4(joins3("optional", var("src[1]"), "is missing")).warn();
            }
            Skip::DestExists => {
                indent4(joins2(var("dest[2]"), "already exists")).warn();
            }
        };

        indent6(bulletdim(src.abs().display())).warn();
        indent6(bulletdim(dest.abs().display())).warn();
    }

    #[inline]
    pub fn error_resolving(copy: bool, src: &CtxPath, dest: &CtxPath, err: Error) {
        match err {
            Error::SrcMissing => {
                indent2(error(joins4(
                    joins2("couldn't", if copy { "copy" } else { "link" }),
                    ppath(src.rel()),
                    "to",
                    ppath(dest.rel()),
                )))
                .error();

                indent4(contextm(joins2(var("src[1]"), "missing"), ppath(src.rel()))).error();
                indent6(bulletdim(src.abs().display())).error();
                indent6(bulletdim(dest.abs().display())).error();
            }
        }
    }
}
