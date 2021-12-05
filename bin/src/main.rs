mod ctxpath;
mod pretty;

mod load;
mod process;

use std::env;
use std::path::PathBuf;

use clap::Parser;
use directories_next::BaseDirs;
use log::Level;
use stderrlog::ColorChoice;
use shelflib::{action::Action, graph::PackageData};

use crate::pretty::{fatal, output::tl_error};
use crate::process::ProcessOptions;

fn main() {
    let opts = Options::parse();

    if cli(opts).is_err() {
        std::process::exit(1);
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(
    author, version, about,
    license = clap::crate_license!(),
    after_help = concat!("Any and all bug reports and contributors are greatly appreciated at ",
                         env!("CARGO_PKG_REPOSITORY"), "!")
)]
pub struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    pub verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    pub quiet: bool,

    #[clap(short, long, about = "Pretend to process")]
    pub noop: bool,

    #[clap(
        short,
        long,
        about = "Linking destination (defaults to home directory)"
    )]
    pub home: Option<String>,

    #[clap(required = true)]
    pub packages: Vec<String>,
}

#[inline]
pub fn cli(opts: Options) -> Result<(), ()> {
    stderrlog::new()
        .quiet(opts.quiet)
        .verbosity(opts.verbosity + 2)
        .show_level(false)
        .color(ColorChoice::Never)
        .init()
        .unwrap();

    run(opts).map_err(|_| tl_error(fatal("errors were encountered; see above.")))
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    let packages: Vec<_> = opts
        .packages
        .iter()
        .map(|path| PathBuf::from(path))
        .collect();

    let (graph, pm) = load::load(packages)?;

    let process_opts = process_opts(opts)?;
    process::process(&graph, &pm, process_opts)?;

    Ok(())
}

#[inline]
fn process_opts(opts: Options) -> Result<ProcessOptions, ()> {
    match BaseDirs::new() {
        Some(bd) => {
            let dest = opts
                .home
                .map(PathBuf::from)
                .unwrap_or_else(|| bd.home_dir().to_path_buf());

            Ok(ProcessOptions {
                noop: opts.noop,
                dest,
            })
        }
        None => {
            tl_error(fatal("couldn't determine home directory; try --home"));
            Err(())
        }
    }
}
