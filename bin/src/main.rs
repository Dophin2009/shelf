#[macro_use]
mod output;

mod load;

use std::env;
use std::process;

use bunt_logger::{ColorChoice, Level};
use clap::Parser;

fn main() {
    let opts = Options::parse();

    bunt_logger::with()
        .quiet(opts.quiet)
        .level(match opts.verbosity {
            0 => Level::Info,
            1 => Level::Debug,
            _ => Level::Trace,
        })
        .stderr(ColorChoice::Auto);

    if cli(opts).is_err() {
        process::exit(1);
    }
}

#[derive(Parser, Debug)]
#[clap(version = clap::crate_version!(), author = clap::crate_authors!(), about = clap::crate_description!())]
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
    run(opts).map_err(|_| tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}"))
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    tl_info!("Loading packages");
    let _graph = crate::load::load(&opts.packages);

    Ok(())
}
