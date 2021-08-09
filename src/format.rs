#![allow(dead_code)]
use std::fmt::Display;

use console::StyledObject;

pub use console::style;

macro_rules! leveled {
    ($name:ident) => {
        leveled!($name, error);
        leveled!($name, warn);
        leveled!($name, info);
        leveled!($name, debug);
        leveled!($name, trace);
    };
    ($name:ident, $level:ident) => {
        #[allow(dead_code)]
        #[inline]
        pub fn $level<M>(message: M)
        where
            M: Display,
        {
            $name::default().$level(message);
        }
    };
}

macro_rules! leveledm {
    () => {
        leveledm!(error);
        leveledm!(warn);
        leveledm!(info);
        leveledm!(debug);
        leveledm!(trace);
    };
    ($level:ident) => {
        #[allow(dead_code)]
        #[inline]
        pub fn $level<M>(&self, message: M)
        where
            M: Display,
        {
            $level!("{}", &self.format(message));
        }
    };
}

#[inline]
pub fn filepath<D>(path: D) -> StyledObject<D>
where
    D: Display,
{
    style(path).green()
}

pub mod toplevel {
    use std::fmt::Display;

    use console::{style, StyledObject};
    use log::{debug, error, info, trace, warn};
    use once_cell::sync::OnceCell;

    leveled!(Toplevel);

    pub struct Toplevel<D>
    where
        D: Display,
    {
        prefix: D,
    }

    impl Toplevel<StyledObject<&'static str>> {
        #[inline]
        pub fn default() -> &'static Self {
            static INSTANCE: OnceCell<Toplevel<StyledObject<&'static str>>> = OnceCell::new();
            INSTANCE.get_or_init(|| Self::new(style("==>").bold().dim()))
        }
    }

    impl<D> Toplevel<D>
    where
        D: Display,
    {
        #[inline]
        pub fn new(prefix: D) -> Self {
            Self { prefix }
        }

        #[inline]
        pub fn format<M>(&self, message: M) -> String
        where
            M: Display,
        {
            format!("{} {}", self.prefix, message)
        }

        leveledm!();
    }
}

pub mod sublevel {
    use std::fmt::Display;

    use console::{style, StyledObject};
    use log::{debug, error, info, trace, warn};
    use once_cell::sync::OnceCell;

    leveled!(Sublevel);

    pub struct Sublevel<D>
    where
        D: Display,
    {
        prefix: D,
    }

    impl Sublevel<StyledObject<&'static str>> {
        #[inline]
        pub fn default() -> &'static Self {
            static INSTANCE: OnceCell<Sublevel<StyledObject<&'static str>>> = OnceCell::new();
            INSTANCE.get_or_init(|| Self::new(style(">").bold().dim()))
        }
    }

    impl<D> Sublevel<D>
    where
        D: Display,
    {
        #[inline]
        pub fn new(prefix: D) -> Self {
            Self { prefix }
        }

        #[inline]
        pub fn format<M>(&self, message: M) -> String
        where
            M: Display,
        {
            format!("  {} {}", self.prefix, message)
        }

        leveledm!();
    }
}

pub mod errored {
    use std::fmt::Display;

    use console::{style, StyledObject};
    use log::{debug, error, info, trace, warn};
    use once_cell::sync::OnceCell;

    use super::sublevel::Sublevel;

    leveled!(Errored);

    pub struct Errored<D>
    where
        D: Display,
    {
        sublevel: Sublevel<D>,
    }

    impl Errored<StyledObject<&'static str>> {
        #[inline]
        pub fn default() -> &'static Self {
            static INSTANCE: OnceCell<Errored<StyledObject<&'static str>>> = OnceCell::new();
            INSTANCE.get_or_init(|| Self::new(style(">").bold().dim().red()))
        }
    }

    impl<D> Errored<D>
    where
        D: Display,
    {
        #[inline]
        pub fn new(prefix: D) -> Self {
            Self {
                sublevel: Sublevel::new(prefix),
            }
        }

        #[inline]
        pub fn format<M>(&self, message: M) -> String
        where
            M: Display,
        {
            format!("{}", self.sublevel.format(message))
        }

        leveledm!();
    }
}

pub mod indexed {
    use std::fmt::Display;
    use std::iter;

    use console::style;
    use log::{debug, error, info, trace, warn};

    #[derive(Debug, Clone)]
    pub struct Indexed {
        c: usize,
        n: usize,
        max_padding: usize,
    }

    impl Indexed {
        #[inline]
        pub fn new(n: usize) -> Self {
            Self {
                c: 1,
                n,
                max_padding: n.to_string().len(),
            }
        }

        #[inline]
        pub fn incr(&mut self) {
            self.c += 1;
        }

        #[inline]
        pub fn format<M>(&self, message: M) -> String
        where
            M: Display,
        {
            let padding = iter::repeat(' ')
                .take(self.max_padding - self.c.to_string().len())
                .collect::<String>();

            let idx = format!("[{}{}/{}]", padding, self.c, self.n);
            format!("{} {}", style(idx).dim(), style(message))
        }

        leveledm!();
    }
}
