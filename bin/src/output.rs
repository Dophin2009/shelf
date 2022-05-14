// TODO: Efficiency of this stuff is probably awful.
use std::fmt::Display;
use std::path::Path;

pub use self::comb::{Prettify, Pretty};

#[derive(Debug, Clone, Copy)]
pub struct Section;

impl Section {
    #[inline]
    pub fn message(word: impl Display, rest: impl Display) {
        log::info!("{} {}", comb::pretty(word).dim(), rest);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Step;

impl Step {
    #[inline]
    pub fn message(message: impl Display) {
        let prefix = comb::indent(2, "->").dim();
        log::debug!("{} {}", prefix, message);
    }
}

macro_rules! Prefixes {
    (
        $Name:ident, $name:ident, $prefix:expr,
        $context_prefix:expr, $reason_prefix:expr, $log:path
    ) => {
        #[allow(dead_code)]
        impl Section {
            #[inline]
            pub fn $name() -> $Name<0> {
                $Name
            }
        }

        #[allow(dead_code)]
        impl Step {
            #[inline]
            pub fn $name() -> $Name<5> {
                $Name
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $Name<const I: usize>;

        #[allow(dead_code)]
        impl<const I: usize> $Name<I> {
            #[inline]
            pub fn message(&self, message: impl Display) -> Self {
                let message = comb::sjoin2(Self::prefix(), message);
                let message = comb::indent(I, message);
                self.print(message);
                self.clone()
            }

            #[inline]
            pub fn context(&self, message: impl Display) -> Self {
                let message = comb::sjoin2(Self::context_prefix(), message);
                let message = comb::indent(I, message);
                self.print(message);
                self.clone()
            }

            #[inline]
            pub fn reason(&self, message: impl Display) -> Self {
                let message = comb::sjoin2(Self::reason_prefix(), message);
                let message = comb::indent(I, message);
                self.print(message);
                self.clone()
            }

            #[inline]
            fn print(&self, message: impl Display) {
                $log!("{}", message)
            }

            #[inline]
            fn prefix() -> Pretty {
                $prefix
            }

            #[inline]
            fn context_prefix() -> Pretty {
                $context_prefix
            }

            #[inline]
            fn reason_prefix() -> Pretty {
                $reason_prefix
            }
        }
    };
}

Prefixes!(
    Fatal,
    fatal,
    comb::pretty("fatal: ").red().bold(),
    comb::pretty(" when  ").dim(),
    comb::pretty("   as  ").dim(),
    log::error
);

Prefixes!(
    Error,
    error,
    comb::pretty("error: ").red().bold(),
    comb::pretty(" when  ").dim(),
    comb::pretty("   as  ").dim(),
    log::error
);

Prefixes!(
    Warning,
    warning,
    comb::pretty(" warn: ").yellow().bold(),
    comb::pretty(" when  ").dim(),
    comb::pretty("   as  ").dim(),
    log::warn
);

Prefixes!(
    Note,
    note,
    comb::pretty(" note: ").dim().bold(),
    comb::pretty(" when  ").dim(),
    comb::pretty("   as  ").dim(),
    log::warn
);

Prefixes!(
    Skipping,
    skipping,
    comb::pretty(" skip: ").blue().bold(),
    comb::pretty(" when  ").dim(),
    comb::pretty("   as  ").dim(),
    log::warn
);

// TODO: Separate colors for source and destination paths?
#[inline]
pub fn spath(path: impl AsRef<Path>) -> Pretty {
    comb::pretty(path.as_ref().display()).green()
}

#[allow(dead_code)]
pub mod comb {
    use std::fmt::Display;

    use crossterm::style::StyledContent;

    pub use crossterm::style::Stylize as Prettify;
    pub type Pretty = StyledContent<String>;

    #[inline]
    pub fn pretty(d: impl Display) -> Pretty {
        crossterm::style::style(format!("{}", d))
    }

    #[inline]
    pub fn empty() -> Pretty {
        pretty("")
    }

    /// A new-line prepended message.
    #[inline]
    pub fn newline(d: impl Display) -> Pretty {
        pretty(format!("\n{}", d))
    }

    /// A new-line appended message.
    #[inline]
    pub fn endline(d: impl Display) -> Pretty {
        pretty(format!("{}\n", d))
    }

    /// A surrounded message.
    #[inline]
    pub fn surround(s: impl Display, d: impl Display) -> Pretty {
        pretty(format!("{}{}{}", s, d, s))
    }

    /// A backticked message.
    #[inline]
    pub fn tick(d: impl Display) -> Pretty {
        surround('`', d)
    }

    /// A single-quoted message.
    #[inline]
    pub fn squote(d: impl Display) -> Pretty {
        surround('"', d)
    }

    /// A double-quoted message.
    #[inline]
    pub fn quote(d: impl Display) -> Pretty {
        surround('\'', d)
    }

    /// A parenthesized message.
    #[inline]
    pub fn paren(d: impl Display) -> Pretty {
        pretty(format!("({})", d))
    }

    #[inline]
    pub fn bracket(d: impl Display) -> Pretty {
        pretty(format!("[{}]", d))
    }

    #[inline]
    pub fn brace(d: impl Display) -> Pretty {
        pretty(format!("{{{}}}", d))
    }

    #[inline]
    pub fn indent(n: usize, d: impl Display) -> Pretty {
        pretty(format!("{:n$}{}", "", d, n = n))
    }

    /// Join 2 strings together with space.
    #[inline]
    pub fn sjoin2(d1: impl Display, d2: impl Display) -> Pretty {
        pretty(format!("{} {}", d1, d2))
    }

    /// Join 3 strings together with spaces.
    #[inline]
    pub fn sjoin3(d1: impl Display, d2: impl Display, d3: impl Display) -> Pretty {
        pretty(format!("{} {} {}", d1, d2, d3))
    }

    /// Join 4 strings together with spaces.
    #[inline]
    pub fn sjoin4(
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty {
        pretty(format!("{} {} {} {}", d1, d2, d3, d4))
    }

    /// Join 2 strings together with a separator.
    #[inline]
    pub fn join2(s: impl Display, d1: impl Display, d2: impl Display) -> Pretty {
        pretty(format!("{}{}{}", d1, s, d2))
    }

    /// Join 3 strings together with a separator.
    #[inline]
    pub fn join3(s: impl Display, d1: impl Display, d2: impl Display, d3: impl Display) -> Pretty {
        pretty(format!("{}{}{}{}{}", d1, s, d2, s, d3))
    }

    /// Join 4 strings together with a separator.
    #[inline]
    pub fn join4(
        s: impl Display,
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty {
        pretty(format!("{}{}{}{}{}{}{}", d1, s, d2, s, d3, s, d4))
    }

    /// Concatenate 2 strings together.
    #[inline]
    pub fn concat2(d1: impl Display, d2: impl Display) -> Pretty {
        pretty(format!("{}{}", d1, d2))
    }

    /// Concatenate 3 strings together.
    #[inline]
    pub fn concat3(d1: impl Display, d2: impl Display, d3: impl Display) -> Pretty {
        pretty(format!("{}{}{}", d1, d2, d3))
    }

    /// Concatenate 4 strings together.
    #[inline]
    pub fn concat4(
        d1: impl Display,
        d2: impl Display,
        d3: impl Display,
        d4: impl Display,
    ) -> Pretty {
        pretty(format!("{}{}{}{}", d1, d2, d3, d4))
    }
}
