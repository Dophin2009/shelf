#[macro_use]
pub mod error;

#[macro_use]
pub mod prettylog;

pub mod action;
pub mod cache;
pub mod cli;
pub mod graph;
pub mod link;
pub mod load;
pub mod op;
pub mod pathutil;
pub mod spec;
pub mod templating;
pub mod tree;

use std::process;

use bunt_logger::{ColorChoice, Level};
use clap::Clap;

use crate::cli::Options;

fn main() {
    let opts = Options::parse();

    bunt_logger::with()
        .quiet(opts.quiet)
        .level(match opts.verbosity {
            0 => Level::Info,
            1 => Level::Debug,
            _ => Level::Trace,
        })
        .stderr(ColorChoice::Auto);

    if cli::cli(opts).is_err() {
        process::exit(1);
    }
}
