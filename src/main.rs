mod config;
mod map;
mod package;

use package::{Linker, Package};

use std::env;
use std::process;

use anyhow::Result;
use clap::Clap;
use log::error;
use stderrlog::ColorChoice;

#[derive(Clap, Debug)]
#[clap(version = "0.1.0", author = "Eric Zhao <21zhaoe@protonmail.com>")]
struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    quiet: bool,
    package: String,
}

fn main() {
    let opts = Options::parse();

    stderrlog::new()
        .show_level(false)
        .quiet(opts.quiet)
        .verbosity(opts.verbosity)
        .color(ColorChoice::Never)
        .init()
        .unwrap();

    let exit = match cli(&opts) {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    };
    process::exit(exit);
}

fn cli(opts: &Options) -> Result<()> {
    let cwd = env::current_dir()?;
    let package_path = cwd.join(&opts.package);

    let package_config = Package::from_path(package_path)?;
    let linker = Linker::new(package_config, opts.quiet, opts.verbosity);

    linker.link()?;

    Ok(())
}
