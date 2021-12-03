#[macro_use]
mod output;
mod cli;
mod pathutil;

use std::process;

use bunt_logger::{ColorChoice, Level};
use clap::Parser;

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
