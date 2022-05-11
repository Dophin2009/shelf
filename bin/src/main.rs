#![allow(clippy::result_unit_err)]

mod ctxpath;
mod output;

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

use crate::load::Loader;
use crate::output::{Prettify, Section};
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

    run(opts)
        .map_err(|_| Section::fatal().message("errors were encountered; see above"))
        .map_err(|_| ())
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    let packages: Vec<_> = opts.packages.iter().map(PathBuf::from).collect();
    let loaded = Loader::new(packages).load()?;

    // TODO: Load journal from filesystem.
    let mut journal = OpJournal::new();

    let mut processor = Processor::new(process_opts(opts)?, &mut journal);
    processor.process(&loaded.graph, &loaded.paths)?;

    Section::message("", "");
    Section::message("done:".green().bold(), "no issues encountered");

    Ok(())
}

#[inline]
fn process_opts(opts: Options) -> Result<ProcessorOptions, ()> {
    let bd = Lazy::new(BaseDirs::new);

    let dest = match opts.home.map(PathBuf::from) {
        Some(home) => {
            // Ensure home directory is absolute.
            let cwd = match env::current_dir() {
                Ok(cwd) => cwd,
                Err(_) => {
                    Section::error().message("couldn't determine current directory");
                    return Err(());
                }
            };
            cwd.join(home)
        }
        None => match &*bd {
            Some(bd) => bd.home_dir().to_path_buf(),
            None => {
                Section::error().message("couldn't determine home directory; try --home");
                return Err(());
            }
        },
    };

    debug_assert!(dest.is_absolute());

    // TODO: No journal option.
    let file_safe_path = match &*bd {
        Some(bd) => {
            // TODO: Extract this logic.
            let timestamp = chrono::offset::Local::now()
                .format("%Y-%m-%d-%H-%M-%S")
                .to_string();
            bd.data_local_dir()
                .join(env!("CARGO_PKG_NAME"))
                .join(timestamp)
        }
        None => {
            Section::error().message("couldn't determine a suitable location for auxiliary data");
            return Err(());
        }
    };
    debug_assert!(file_safe_path.is_absolute());

    let ctx = FinishCtx::new(FileSafe::new(file_safe_path));

    Ok(ProcessorOptions {
        noop: opts.noop,
        dest,
        ctx,
    })
}
