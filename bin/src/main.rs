#[macro_use]
mod output;
mod ctxpath;

mod load;
mod process;

use std::env;
use std::path::PathBuf;

use bunt_logger::{ColorChoice, Level};
use clap::Parser;
use directories_next::BaseDirs;
use shelflib::{action::Action, graph::PackageData};

use crate::process::ProcessOptions;

fn main() {
    let opts = Options::parse();

    if cli(opts).is_err() {
        std::process::exit(1);
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(
    name = clap::crate_name!(),
    version = clap::crate_version!(),
    author = clap::crate_authors!(", "),
    about = clap::crate_description!(),
    license = clap::crate_license!(),
)]
pub struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    pub verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    pub quiet: bool,

    #[clap(short, long, about = "Pretend to process")]
    pub noop: bool,

    #[clap(short, long, about = "Linking destination (defaults to $HOME)")]
    pub home: Option<String>,

    #[clap(required = true)]
    pub packages: Vec<String>,
}

#[inline]
pub fn cli(opts: Options) -> Result<(), ()> {
    bunt_logger::with()
        .quiet(opts.quiet)
        .level(match opts.verbosity {
            0 => Level::Info,
            1 => Level::Debug,
            _ => Level::Trace,
        })
        .stderr(ColorChoice::Auto);

    run(opts).map_err(|_| tl_error!("{$red+bold}fatal:{/$} errors were encountered! See above."))
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
        None => Err(()),
    }
}
