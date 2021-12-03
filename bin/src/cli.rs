use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;

use clap::Parser;
use directories_next::BaseDirs;

use shelflib::graph::PackageGraph;
use shelflib::load::SpecLoader;
use shelflib::spec::Dep;

use crate::pathutil::PathWrapper;

const NAME: &str = "tidy";

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
    Ok(())
}
