use std::env;

use clap::Parser;

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

    #[clap(long, about = "Use an alternate cache location")]
    pub cache: Option<String>,
    #[clap(long, about = "Don't cache actions")]
    pub no_cache: bool,
    #[clap(long, about = "Clear any existing cache")]
    pub clear_cache: bool,

    pub packages: Vec<String>,
}

#[inline]
pub fn cli(opts: Options) -> Result<(), ()> {
    run(opts).map_err(|_| tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}"))
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    if opts.packages.is_empty() {
        tl_info!("No packages specified, exiting");
        return Ok(());
    }

    tl_info!("Loading packages");
    let _graph = crate::load::load(&opts.packages);

    Ok(())
}
