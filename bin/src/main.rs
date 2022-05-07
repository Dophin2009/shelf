#![allow(clippy::result_unit_err)]

pub mod ctxpath;
pub mod pretty;

mod load;
mod process;

use std::env;
use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use directories_next::BaseDirs;
use once_cell::unsync::Lazy;
use shelflib::op::{
    ctx::{FileSafe, FinishCtx},
    journal::OpJournal,
};
use stderrlog::ColorChoice;

use crate::pretty::{
    nline,
    output::Emit,
    semantic::{error, fatal},
};
use crate::process::{Processor, ProcessorOptions};

fn main() {
    let opts = Options::parse();

    if cli(opts).is_err() {
        std::process::exit(1);
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(
    author, version, about,
    after_help = concat!("Any and all bug reports and contributions are greatly appreciated at ",
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
    let loaded = load::load(packages)?;

    // TODO: Load journal from filesystem.
    let mut journal = OpJournal::new();

    let mut processor = Processor::new(process_opts(opts)?, &mut journal);
    processor.process(&loaded.graph, &loaded.paths)
}

#[inline]
fn process_opts(opts: Options) -> Result<ProcessorOptions, ()> {
    let bd = Lazy::new(BaseDirs::new);

    let dest = match opts.home.map(PathBuf::from) {
        Some(home) => home,
        None => match &*bd {
            Some(bd) => bd.home_dir().to_path_buf(),
            None => {
                error("couldn't determine home directory; try --home").error();
                return Err(());
            }
        },
    };

    // TODO: No journal option.
    let file_safe_path = match &*bd {
        Some(bd) => bd.data_local_dir().to_path_buf(),
        None => {
            error("couldn't determine a suitable location for auxiliary data").error();
            return Err(());
        }
    };
    let ctx = FinishCtx::new(FileSafe::new(file_safe_path));

    Ok(ProcessorOptions {
        noop: opts.noop,
        dest,
        ctx,
    })
}
