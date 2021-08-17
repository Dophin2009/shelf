use std::env;
use std::path::PathBuf;

use clap::Clap;

use crate::action::{Resolve, ResolveOpts};
use crate::error::EmptyError;
use crate::link;
use crate::load;
use crate::pathutil::PathWrapper;

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
            tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}");
            Err(err)
        }
    }
}

fn run(opts: Options) -> Result<(), EmptyError> {
    // FIXME error printing
    let dest = fail!(get_dest(opts.home));

    tl_info!("Loading packages...");
    let graph = fail!(load::load_multi(&opts.packages));

    tl_info!("Sorting packages...");
    let packages = fail!(link::link(&dest, &graph));

    tl_info!("Starting package linking...");
    for actions in packages {
        tl_info!("Linking {$blue}{}{/$}...", actions.name());
        for action in actions {
            // FIXME support for choosing fail-fast/skip/etc. on error
            fail!(action.resolve(&ResolveOpts {}));
        }
    }

    Ok(())
}

#[inline]
fn get_dest<'a>(opt: Option<String>) -> Result<PathWrapper, EmptyError> {
    let path = match opt {
        Some(p) => PathBuf::from(p),
        None => match home::home_dir() {
            Some(p) => p,
            None => {
                tl_error!("Couldn't determine home directory");
                return Err(EmptyError);
            }
        },
    };

    Ok(PathWrapper::from_cwd(path))
}
