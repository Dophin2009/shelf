#![allow(clippy::result_unit_err)]

pub mod ctxpath;
pub mod pretty;

mod load;
mod process;

use std::env;
use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use directories_next::BaseDirs;
use stderrlog::ColorChoice;

use crate::pretty::{
    nline,
    output::Emit,
    semantic::{error, fatal},
};
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
    after_help = concat!("Any and all bug reports and contributors are greatly appreciated at ",
                         env!("CARGO_PKG_REPOSITORY"), "!")
)]
#[clap(group(
    ArgGroup::new("vers")
        .args(&["verbosity", "quiet"]),
))]
pub struct Options {
    #[clap(short, long, parse(from_occurrences), help = "Message verbosity")]
    pub verbosity: usize,
    #[clap(short, long, help = "Silence all output")]
    pub quiet: bool,

    #[clap(short, long, help = "Pretend to process")]
    pub noop: bool,

    #[clap(long, help = "Set linking destination")]
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

    run(opts).map_err(|_| nline(fatal("errors were encountered; see above")).error())
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    let packages: Vec<_> = opts.packages.iter().map(PathBuf::from).collect();

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
            error("couldn't determine home directory; try --home").error();
            Err(())
        }
    }
}
