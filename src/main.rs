#[macro_use]
pub mod error;

pub mod action;
pub mod cli;
pub mod format;
pub mod graph;
pub mod link;
pub mod load;
pub mod spec;
pub mod templating;
pub mod tree;

use std::process;

use clap::Clap;
use stderrlog::ColorChoice;

use crate::cli::Options;

fn main() {
    let opts = Options::parse();

    stderrlog::new()
        .show_level(false)
        .quiet(opts.quiet)
        .verbosity(opts.verbosity + 2)
        .color(ColorChoice::Never)
        .module(module_path!())
        .init()
        .unwrap();

    if cli::cli(opts).is_err() {
        process::exit(1);
    }
}
