use std::iter;

use log::{debug, error, info, trace, warn};

pub trait Logger {
    fn format(&self, message: &str) -> String;
}

pub struct Toplevel;

impl Toplevel {
    #[inline]
    pub fn format(&self, message: &str) -> String {
        format!("=> {}", message)
    }
}

impl Logger for Toplevel {
    #[inline]
    fn format(&self, message: &str) -> String {
        self.format(message)
    }
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
    pub fn format(&self, message: &str) -> String {
        let padding = iter::repeat(' ')
            .take(self.max_padding - self.c.to_string().len())
            .collect::<String>();

        format!("[{}{}/{}] {}", padding, self.c, self.n, message)
    }
}

impl Logger for Indexed {
    #[inline]
    fn format(&self, message: &str) -> String {
        self.format(message)
    }
}
