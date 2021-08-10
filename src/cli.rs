use std::env;
use std::path::PathBuf;

use clap::Clap;

use crate::action::{Resolve, ResolveOpts};
use crate::error::EmptyError;
use crate::format::{
    style,
    toplevel::{self, Toplevel},
};
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

pub fn cli(opts: Options) -> Result<(), EmptyError> {
    match run(opts) {
        Ok(_) => Ok(()),
        Err(err) => {
            Toplevel::new(style("==>").bold().red()).error(
                style("Fatal errors were encountered! See above.")
                    .bold()
                    .red(),
            );
            Err(err)
        }
    }
}

fn run(opts: Options) -> Result<(), EmptyError> {
    // FIXME error printing
    let dest = fail!(get_dest(opts.home));

    toplevel::info("Loading packages...");
    let graph = fail!(load::load_multi(&opts.packages));

    toplevel::info("Sorting packages...");
    let packages = fail!(link::link(dest, &graph));

    toplevel::info("Starting package linking...");
    for actions in packages {
        toplevel::info(format!("Linking {}...", style(&actions.name).bold().blue()));
        for action in actions {
            // FIXME support for choosing fail-fast/skip/etc. on error
            action.resolve(&ResolveOpts {}).unwrap();
        }
    }

    Ok(())
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
