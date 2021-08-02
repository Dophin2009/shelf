use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Clap;
use stderrlog::ColorChoice;
use tidy::{Linker, Loader, Verbosity};

#[derive(Clap, Debug)]
#[clap(version = clap::crate_version!(), author = "Eric Zhao <21zhaoe@protonmail.com>")]
struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    quiet: bool,
    #[clap(short, long, about = "Pretend to process")]
    noop: bool,

    #[clap(short, long, about = "Linking destination (defaults to $HOME)")]
    home: Option<String>,

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

    cli(opts)
}

static HOME_VAR: &str = "HOME";

fn cli(opts: Options) -> Result<()> {
    let verbosity = if opts.quiet {
        Verbosity::Quiet
    } else if opts.verbosity > 0 {
        Verbosity::Verbose
    } else {
        Verbosity::Info
    };

    let home: PathBuf = match opts.home {
        Some(p) => p,
        None => env::var(HOME_VAR)?,
    }
    .into();

    let loader = Loader::new();
    let graph = loader
        .load_multi(&opts.packages)
        .with_context(|| "Couldn't resolve packages")?;

    let linker = Linker::new(home);
    let actions = linker.link(&graph)?;

    for action in actions {
        action.resolve()?;
    }

    Ok(())
}
