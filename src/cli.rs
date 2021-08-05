use std::env;
use std::path::PathBuf;

use clap::Clap;

use crate::action::{Resolvable, ResolveOpts};
use crate::link;
use crate::load;

#[derive(Clap, Debug)]
#[clap(version = clap::crate_version!(), author = "Eric Zhao <21zhaoe@protonmail.com>")]
pub struct Options {
    #[clap(short, long, parse(from_occurrences), about = "Message verbosity")]
    pub verbosity: usize,
    #[clap(short, long, about = "Silence all output")]
    pub quiet: bool,
    #[clap(short, long, about = "Pretend to process")]
    pub noop: bool,

    #[clap(short, long, about = "Linking destination (defaults to $HOME)")]
    pub home: Option<String>,

    pub packages: Vec<String>,
}

pub fn cli(opts: Options) {
    // FIXME error handling
    let dest = get_dest(opts.home).unwrap();

    // FIXME error handling
    let graph = load::load_multi(&opts.packages).unwrap();

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
    static HOME_VAR: &str = "HOME";
    let ret = match home {
        Some(p) => p,
        None => env::var(HOME_VAR)?,
    }
    .into();

    Ok(ret)
}
