pub mod output;
pub mod semantic;

use std::fmt::Display;
use std::iter;
use std::path::Path;

// Re-export core style types.
pub use crossterm::style::{style as pretty, StyledContent as Pretty, Stylize as Prettify};

/// A surrounded message.
#[inline]
pub fn surround<D1: Display, D2: Display>(s: D1, d: D2) -> Pretty<String> {
    pretty(format!("{}{}{}", s, d, s))
}

/// A backticked message.
#[inline]
pub fn tick<D: Display>(d: D) -> Pretty<String> {
    surround('`', d)
}

/// A single-quoted message.
#[inline]
pub fn squote<D: Display>(d: D) -> Pretty<String> {
    surround('"', d)
}

/// A double-quoted message.
#[inline]
pub fn quote<D: Display>(d: D) -> Pretty<String> {
    surround('\'', d)
}

/// A parenthesized message.
#[inline]
pub fn paren<D: Display>(d: D) -> Pretty<String> {
    join3('(', d, ')')
}

#[inline]
pub fn bracket<D: Display>(d: D) -> Pretty<String> {
    join3('[', d, ']')
}

#[inline]
pub fn brace<D: Display>(d: D) -> Pretty<String> {
    join3('{', d, '}')
}

/// Join 2 strings together with space.
#[inline]
pub fn join2<D1: Display, D2: Display>(d1: D1, d2: D2) -> Pretty<String> {
    joins2(' ', d1, d2)
}

/// Join 3 strings together with spaces.
#[inline]
pub fn join3<D1: Display, D2: Display, D3: Display>(d1: D1, d2: D2, d3: D3) -> Pretty<String> {
    joins3(' ', d1, d2, d3)
}

/// Join 4 strings together with spaces.
#[inline]
pub fn join4<D1: Display, D2: Display, D3: Display, D4: Display>(
    d1: D1,
    d2: D2,
    d3: D3,
    d4: D4,
) -> Pretty<String> {
    joins4(' ', d1, d2, d3, d4)
}

/// Join 2 strings together with a separator.
#[inline]
pub fn joins2<S: Display, D1: Display, D2: Display>(s: S, d1: D1, d2: D2) -> Pretty<String> {
    pretty(format!("{}{}{}", d1, s, d2))
}

/// Join 3 strings together with a separator.
#[inline]
pub fn joins3<S: Display, D1: Display, D2: Display, D3: Display>(
    s: S,
    d1: D1,
    d2: D2,
    d3: D3,
) -> Pretty<String> {
    let d12 = joins2(&s, d1, d2);
    joins2(s, d12, d3)
}

/// Join 4 strings together with a separator.
#[inline]
pub fn joins4<S: Display, D1: Display, D2: Display, D3: Display, D4: Display>(
    s: S,
    d1: D1,
    d2: D2,
    d3: D3,
    d4: D4,
) -> Pretty<String> {
    let d123 = joins3(&s, d1, d2, d3);
    joins2(s, d123, d4)
}

/// Indent 1 space.
#[inline]
pub fn indent1<D: Display>(d: D) -> Pretty<String> {
    pretty(format!(" {}", d))
}

/// Indent 2 spaces.
#[inline]
pub fn indent2<D: Display>(d: D) -> Pretty<String> {
    indentn(2, d)
}

/// Indent 4 spaces.
#[inline]
pub fn indent4<D: Display>(d: D) -> Pretty<String> {
    indentn(4, d)
}

/// Indent 6 spaces.
#[inline]
pub fn indent6<D: Display>(d: D) -> Pretty<String> {
    indentn(6, d)
}

/// Indent 8 spaces.
#[inline]
pub fn indent8<D: Display>(d: D) -> Pretty<String> {
    indentn(8, d)
}

#[inline]
pub fn indentn<D: Display>(n: usize, d: D) -> Pretty<String> {
    repeat(n, indent1, format!("{}", d))
}

#[inline]
pub fn repeat<D: Display, F>(n: usize, f: F, d: D) -> Pretty<D>
where
    F: Clone + FnOnce(Pretty<D>) -> Pretty<D>,
{
    iter::repeat(f).take(n).fold(pretty(d), |acc, f| f(acc))
}
