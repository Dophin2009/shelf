use stew::dependency::PackageGraph;
use stew::linker::Linker;
use stew::package::Package;

use std::env;

use anyhow::{anyhow, Context, Result};
use clap::Clap;
use log::info;
use stderrlog::ColorChoice;

#[derive(Clap, Debug)]
#[clap(version = "0.1.0", author = "Eric Zhao <21zhaoe@protonmail.com>")]
struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    quiet: bool,
    packages: Vec<String>,
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
        Ok(val) => val,
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

    let linker = Linker::new(home, opts.quiet, opts.verbosity);
    let mut graph = PackageGraph::new();

    info!("Resolving package dependency graph...");
    for package in &opts.packages {
        let path = cwd.join(package);
        // let package = Package::from_directory(&path)?;

        // graph.add_package(path, package)?;
    }

    linker.link_package_graph(&graph)?;

    Ok(())
}
