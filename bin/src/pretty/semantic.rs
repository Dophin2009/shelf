use std::fmt::Display;
use std::path::Path;

use super::{concat2, joins2, pretty, string, Prettify, Pretty};

/// A fatal error message.
#[inline]
pub fn fatal<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("fatal:").red().bold(), d)
}

/// A non-fatal error message.
#[inline]
pub fn error<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("error:").red().bold(), d)
}

/// A warning message.
#[inline]
pub fn warning<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("warning:").dark_yellow().bold(), d)
}

/// A skip warning.
#[inline]
pub fn skipping<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("skipping:").dark_yellow().bold(), d)
}

#[inline]
pub fn info<D: Display>(d: D) -> Pretty<String> {
    string(d)
}

#[inline]
pub fn context<D: Display>(d: D) -> Pretty<String> {
    concat2(d, ":")
}

#[inline]
pub fn contextm<D1: Display, D2: Display>(d1: D1, d2: D2) -> Pretty<String> {
    joins2(context(d1), d2)
}

#[inline]
pub fn arrow<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("->").dim(), d)
}

#[inline]
pub fn arrowdim<D: Display>(d: D) -> Pretty<String> {
    joins2("->", d).dim()
}

#[inline]
pub fn bullet<D: Display>(d: D) -> Pretty<String> {
    joins2(pretty("-").dim(), d)
}

#[inline]
pub fn bulletdim<D: Display>(d: D) -> Pretty<String> {
    joins2("-", d).dim()
}

/// A path.
#[inline]
pub fn path<D: Display>(d: D) -> Pretty<String> {
    pretty(format!("{}", d)).green().dim()
}

/// A path (with actual path argument).
#[inline]
pub fn ppath<P: AsRef<Path>>(p: P) -> Pretty<String> {
    path(p.as_ref().display())
}

/// A variable.
#[inline]
pub fn var<D: Display>(d: D) -> Pretty<D> {
    pretty(d).blue()
}
