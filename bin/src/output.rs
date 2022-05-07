// TODO: Efficiency of this stuff is probably awful.
use std::fmt::Display;
use std::path::Path;

pub use crossterm::style::{style as pretty, StyledContent as Pretty, Stylize as Prettify};

#[inline]
pub fn path(path: impl AsRef<Path>) -> Pretty<String> {
    comb::string(path.as_ref().display()).green()
}

#[inline]
pub fn fatal(message: impl Display) {
    let prefix = comb::string("fatal:").bold().red();
    log::error!("\n{} {}", prefix, message);
}

#[inline]
pub fn section(word: impl Display, rest: impl Display) {
    let word = comb::string(word).dim();
    log::info!("{} {}", word, rest);
}

#[inline]
pub fn section_error(message: impl Display) {
    log::error!("{} {}", error_prefix(), message);
}

#[inline]
pub fn step(message: impl Display) {
    let arrow = comb::string("->").dim();
    log::debug!("{} {}", comb::indent(2, arrow), message)
}

#[inline]
pub fn step_error_context(message: impl Display) {
    let prefix = comb::string("in:").dim().bold();
    log::error!("{} {}", comb::indent(4, prefix), message);
}

#[inline]
pub fn step_error(message: impl Display) {
    log::error!("{} {}", comb::indent(4, error_prefix()), message);
}

#[inline]
fn error_prefix() -> Pretty<String> {
    comb::string("error:").red().bold()
}

#[allow(dead_code)]
pub mod comb {
    use std::fmt::Display;
    use std::iter;

    use super::{pretty, Pretty};

    #[inline]
    pub fn empty() -> Pretty<&'static str> {
        pretty("")
    }

    #[inline]
    pub fn string(d: impl Display) -> Pretty<String> {
        pretty(format!("{}", d))
    }

    /// A new-line prepended message.
    #[inline]
    pub fn newline(d: impl Display) -> Pretty<String> {
        concat2('\n', d)
    }

    /// A new-line appended message.
    #[inline]
    pub fn line(d: impl Display) -> Pretty<String> {
        concat2(d, '\n')
    }

    /// A surrounded message.
    #[inline]
    pub fn surround(s: impl Display, d: impl Display) -> Pretty<String> {
        let d = concat2(&s, d);
        concat2(d, s)
    }

    /// A backticked message.
    #[inline]
    pub fn tick(d: impl Display) -> Pretty<String> {
        surround('`', d)
    }

    /// A single-quoted message.
    #[inline]
    pub fn squote(d: impl Display) -> Pretty<String> {
        surround('"', d)
    }

    /// A double-quoted message.
    #[inline]
    pub fn quote(d: impl Display) -> Pretty<String> {
        surround('\'', d)
    }

    /// A parenthesized message.
    #[inline]
    pub fn paren(d: impl Display) -> Pretty<String> {
        concat3('(', d, ')')
    }

    #[inline]
    pub fn bracket(d: impl Display) -> Pretty<String> {
        concat3('[', d, ']')
    }

    #[inline]
    pub fn brace(d: impl Display) -> Pretty<String> {
        concat3('{', d, '}')
    }

    #[inline]
    pub fn indent(n: usize, d: impl Display) -> Pretty<String> {
        repeat(n, |d| sjoin2("", d), format!("{}", d))
    }

    /// Join 2 strings together with space.
    #[inline]
    pub fn sjoin2(d1: impl Display, d2: impl Display) -> Pretty<String> {
        join2(' ', d1, d2)
    }

    /// Join 3 strings together with spaces.
    #[inline]
    pub fn sjoin3(d1: impl Display, d2: impl Display, d3: impl Display) -> Pretty<String> {
        sjoin2(sjoin2(d1, d2), d3)
    }

    /// Join 4 strings together with spaces.
    #[inline]
    pub fn sjoin4(
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty<String> {
        sjoin2(sjoin3(d1, d2, d3), d4)
    }

    /// Join 2 strings together with a separator.
    #[inline]
    pub fn join2(s: impl Display, d1: impl Display, d2: impl Display) -> Pretty<String> {
        concat3(d1, s, d2)
    }

    /// Join 3 strings together with a separator.
    #[inline]
    pub fn join3(
        s: impl Display,
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
    ) -> Pretty<String> {
        let d12 = join2(&s, d1, d2);
        join2(s, d12, d3)
    }

    /// Join 4 strings together with a separator.
    #[inline]
    pub fn join4(
        s: impl Display,
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty<String> {
        let d123 = join3(&s, d1, d2, d3);
        join2(s, d123, d4)
    }

    /// Concatenate 2 strings together.
    #[inline]
    pub fn concat2(d1: impl Display, d2: impl Display) -> Pretty<String> {
        pretty(format!("{}{}", d1, d2))
    }

    /// Concatenate 3 strings together.
    #[inline]
    pub fn concat3(d1: impl Display, d2: impl Display, d3: impl Display) -> Pretty<String> {
        concat2(concat2(d1, d2), d3)
    }

    /// Concatenate 4 strings together.
    #[inline]
    pub fn concat4(
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty<String> {
        concat2(concat3(d1, d2, d3), d4)
    }

    #[inline]
    pub fn repeat<F, D: Display>(n: usize, f: F, d: D) -> Pretty<D>
    where
        F: Clone + FnOnce(Pretty<D>) -> Pretty<D>,
    {
        iter::repeat(f).take(n).fold(pretty(d), |acc, f| f(acc))
    }
}
