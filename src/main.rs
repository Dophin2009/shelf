use std::env;

use anyhow::{anyhow, Context, Result};
use clap::Clap;
use log::info;
use stderrlog::ColorChoice;
use tidy::Loader;

#[derive(Clap, Debug)]
#[clap(version = clap::crate_version!(), author = "Eric Zhao <21zhaoe@protonmail.com>")]
struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    quiet: bool,
    #[clap(short, long, about = "Pretend to process")]
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

static HOME_VAR: &str = "HOME";

fn cli(opts: &Options) -> Result<()> {
    let home =
        env::var(HOME_VAR).with_context(|| format!("Environment variable {} not set", HOME_VAR))?;

    let loader = Loader::new();
    let graph = loader
        .load_multi(&opts.packages)
        .with_context(|| "Couldn't resolve packages")?;

    println!(
        "{:#?}",
        graph
            .data()
            .into_iter()
            .map(|(_, ps)| ps.data.clone())
            .collect::<Vec<_>>()
    );

    Ok(())
}
