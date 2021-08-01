pub mod spec;
pub mod tree;

mod action;
mod graph;
mod link;
mod load;

pub use action::*;
pub use link::Linker;
pub use load::Loader;
pub use spec::*;

#[derive(Debug, Clone)]
pub enum Verbosity {
    Quiet,
    Info,
    Verbose,
}
