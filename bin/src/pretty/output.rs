use std::fmt::Display;

use super::{joins2, pretty, Prettify, Pretty};

pub trait Emit: Display {
    #[inline]
    fn error(&self) {
        error(self);
    }

    #[inline]
    fn warn(&self) {
        warn(self);
    }
    #[inline]
    fn info(&self) {
        info(self);
    }

    #[inline]
    fn trace(&self) {
        trace(self);
    }

    #[inline]
    fn debug(&self) {
        debug(self);
    }
}

impl<D> Emit for Pretty<D> where D: Display {}

macro_rules! logfn {
    ($level:ident) => {
        #[inline]
        pub fn $level<D: Display>(d: D) {
            log::$level!("{}", d);
        }
    };
    ($($level:ident),*) => {
        $(logfn!($level);)*
    }
}

macro_rules! xlprefix {
    ($hlevel:ident, $val:literal) => {
        paste::paste! {
            #[inline]
            pub fn [<$hlevel _prefix>]() -> Pretty<&'static str> {
                pretty($val)
            }
        }
    };
}
macro_rules! xlfn {
    ($hlevel:ident, $level:ident; $($modifier:ident),*) => {
        paste::paste! {
            #[inline]
            pub fn [<$hlevel _ $level>]<D: Display>(d: D) {
                let f = joins2([<$hlevel _prefix>]()$(.$modifier())*, d);
                $level(f)
            }
        }
    };
    ($hlevel:ident, $level:ident) => {
        xlfn!($hlevel, $level;);
    };
}

logfn!(error, warn, info, debug, trace);

xlprefix!(tl, "==>");
xlprefix!(sl, "  >");
xlprefix!(sli, "  >  ");
xlprefix!(slii, "  >    ");

xlfn!(tl, error; red, bold);
xlfn!(tl, warn; dark_yellow, bold);
xlfn!(tl, info; dim, bold);
xlfn!(tl, debug; dim);
xlfn!(tl, trace; dim);

xlfn!(sl, error; red, bold);
xlfn!(sl, warn; dark_yellow, bold);
xlfn!(sl, info; dim, bold);
xlfn!(sl, debug; dim);
xlfn!(sl, trace; dim);

xlfn!(sli, error; red, bold);
xlfn!(sli, warn; dark_yellow, bold);
xlfn!(sli, info; dim, bold);
xlfn!(sli, debug; dim);
xlfn!(sli, trace; dim);

xlfn!(slii, error; red, bold);
xlfn!(slii, warn; dark_yellow, bold);
xlfn!(slii, info; dim, bold);
xlfn!(slii, debug; dim);
xlfn!(slii, trace; dim);
