use stew::{Linker, Loader};

use std::env;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use log::info;
use stderrlog::ColorChoice;

#[derive(Parser, Debug)]
#[clap(version = clap::crate_version!(), author = "Eric Zhao <21zhaoe@protonmail.com>")]
struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    quiet: bool,
    #[clap(short, long, about = "Do not link, copy, or write files")]
    noop: bool,
    packages: Vec<String>,
}

fn main() -> Result<()> {
    let opts = Options::parse();

    stderrlog::new()
        .show_level(false)
        .quiet(opts.quiet)
        .verbosity(opts.verbosity)
        .color(ColorChoice::Never)
        .module(module_path!())
        .init()
        .unwrap();

    cli(&opts)
}

fn cli(opts: &Options) -> Result<()> {
    let home_key = "HOME";
    let home = match env::var(home_key) {
        Ok(val) => val,
        Err(err) => {
            return Err(anyhow!(
                "Environment variable {} not set: {}",
                home_key,
                err
            ))
        }
    };

    info!("Resolving package dependency graph...");
    let loader = Loader::new();
    let graph = loader
        .load_multi(&opts.packages)
        .with_context(|| "Failed to resolve packages")?;

    let linker = Linker::new(home, opts.quiet, opts.verbosity);

    if !opts.noop {
        linker
            .link(&graph)
            .with_context(|| "Failed to link packages")?;
    } else {
        linker.link_noop(&graph)?;
    }

    Ok(())
}
