use std::path::PathBuf;

use shelflib::action::link::{Res, Skip};
use shelflib::action::{link::Error, LinkAction, Resolve};

use crate::ctxpath::CtxPath;

#[inline]
pub fn process(action: LinkAction, path: &CtxPath, dest: &PathBuf) -> Result<(), ()> {
    let src = CtxPath::new(&action.src, path.abs()).unwrap();
    let dest = CtxPath::new(&action.dest, dest).unwrap();

    let opstr = if action.copy { "Copying" } else { "Linking" };
    sl_debug!(
        "{} '{[green]}' to '{[green]}'",
        opstr,
        src.rel().display(),
        dest.rel().display(),
    );

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
            sl_i_warn!("{$yellow+bold}skipping:{/$} {$blue}src{/$} and {$blue}dest{/$} are the same");
            sl_ii_warn!("{$dimmed}-> {}{/$}", src.abs().display());
            sl_ii_warn!("{$dimmed}-> {}{/$}", dest.abs().display());
        }
        Skip::OptMissing => {}
        Skip::DestExists => {}
    }
}

#[inline]
fn process_error(src: &CtxPath, _dest: &CtxPath, err: Error) {
    match err {
        Error::SrcMissing => {
            sl_i_error!(
                "{$red+bold}error:{/$} {$blue}src{/$} missing: '{[green]}'",
                src.rel().display()
            );
            sl_ii_error!("{$dimmed}-> {}{/$}", src.abs().display());
        }
    }
}
