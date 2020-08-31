mod config;
mod linker;
mod map;
mod symlink;

use linker::Linker;

use std::env;

use anyhow::{anyhow, Context, Result};
use clap::Clap;
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

fn main() -> Result<()> {
    let opts = Options::parse();

    stderrlog::new()
        .show_level(false)
        .quiet(opts.quiet)
        .verbosity(opts.verbosity)
        .color(ColorChoice::Never)
        .init()
        .unwrap();

    cli(&opts)
}

fn cli(opts: &Options) -> Result<()> {
    let home_key = "HOME";
    let home = match env::var(home_key) {
        Ok(val) => val.into(),
        Err(err) => {
            return Err(anyhow!(
                "Environment variable {} not set: {}",
                home_key,
                err
            ))
        }
    };

    let cwd =
        env::current_dir().with_context(|| "Failed to determine current working directory")?;
    let path = cwd.join(&opts.package);

    let linker = Linker::from_path(path, home, opts.quiet, opts.verbosity)?;

    linker.link()?;

    Ok(())
}
