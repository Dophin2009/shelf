use std::env;
use std::path::PathBuf;

use clap::Clap;
use stderrlog::ColorChoice;
use tidy::{link, Loader, Resolvable, ResolveOpts};

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

    cli(opts);
}

static HOME_VAR: &str = "HOME";

fn cli(opts: Options) {
    // FIXME error handling
    let dest = get_dest(opts.home).unwrap();

    // FIXME error handling
    let loader = Loader::new();
    let graph = loader.load_multi(&opts.packages).unwrap();

    // FIXME error handling
    let packages = link::link(dest, &graph).unwrap();
    for actions in packages {
        for action in actions {
            // FIXME support for choosing fail-fast/skip/etc. on error
            action.resolve(&ResolveOpts {}).unwrap();
        }
    }
}

#[inline]
fn get_dest(home: Option<String>) -> Result<PathBuf, env::VarError> {
    let ret = match home {
        Some(p) => p,
        None => env::var(HOME_VAR)?,
    }
    .into();

    Ok(ret)
}
