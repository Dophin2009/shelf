pub mod output;

use std::fmt::Display;
use std::path::Path;

pub use crossterm::style::{style as pretty, StyledContent as Pretty, Stylize as Prettify};

/// A fatal error message.
#[inline]
pub fn fatal<D: Display>(d: D) -> Pretty<String> {
    join2(pretty("fatal:").red().bold(), d)
}

/// A non-fatal error message.
#[inline]
pub fn error<D: Display>(d: D) -> Pretty<String> {
    join2(pretty("error:").red().bold(), d)
}

/// A warning message.
#[inline]
pub fn warning<D: Display>(d: D) -> Pretty<String> {
    join2(pretty("warning:").dark_yellow().bold(), d)
}

/// A skip warning.
#[inline]
pub fn skipping<D: Display>(d: D) -> Pretty<String> {
    join2(pretty("skipping:").dark_yellow().bold(), d)
}

#[inline]
pub fn arrshow<D: Display>(d: D) -> Pretty<String> {
    join2(pretty("->").dim(), d)
}

#[inline]
pub fn arrshowdim<D: Display>(d: D) -> Pretty<String> {
    join2("->", d).dim()
}

/// A path.
#[inline]
pub fn path<D: Display>(d: D) -> Pretty<String> {
    pretty(format!("{}", d)).green()
}

/// A path (with actual path argument).
#[inline]
pub fn ppath<P: AsRef<Path>>(p: P) -> Pretty<String> {
    path(p.as_ref().display())
}

/// A single-quoted message.
#[inline]
pub fn squoted<D: Display>(d: D) -> Pretty<String> {
    pretty(format!("'{}'", d))
}

/// A double-quoted message.
#[inline]
pub fn quoted<D: Display>(d: D) -> Pretty<String> {
    pretty(format!("\"{}\"", d))
}

/// A variable.
#[inline]
pub fn var<D: Display>(d: D) -> Pretty<D> {
    pretty(d).blue()
}

/// Join 2 strings together with space.
#[inline]
pub fn join2<D1: Display, D2: Display>(d1: D1, d2: D2) -> Pretty<String> {
    pretty(format!("{} {}", d1, d2))
}
