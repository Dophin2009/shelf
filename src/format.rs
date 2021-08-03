use std::fmt::Display;
use std::iter;

use console::{style, StyledObject};
use log::{debug, error, info, trace, warn};
use once_cell::sync::OnceCell;

macro_rules! leveled {
    () => {
        leveled!(error);
        leveled!(warn);
        leveled!(info);
        leveled!(debug);
        leveled!(trace);
    };
    ($level:ident) => {
        #[inline]
        pub fn $level(&self, message: &str) {
            $level!("{}", &self.format(message));
        }
    };
}

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
        INSTANCE.get_or_init(|| Self::new(style("=>").bright()))
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

    leveled!();
}

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

    leveled!();
}

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

    leveled!();
}
