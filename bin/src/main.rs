#[macro_use]
mod output;
mod ctxpath;

mod load;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process;

use bunt_logger::{ColorChoice, Level};
use clap::Parser;
use shelflib::graph::PackageData;

use crate::ctxpath::CtxPath;

fn main() {
    let opts = Options::parse();

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
    bunt_logger::with()
        .quiet(opts.quiet)
        .level(match opts.verbosity {
            0 => Level::Info,
            1 => Level::Debug,
            _ => Level::Trace,
        })
        .stderr(ColorChoice::Auto);

    run(opts).map_err(|_| tl_error!("{$red+bold}Fatal errors were encountered! See above.{/$}"))
}

#[inline]
fn run(opts: Options) -> Result<(), ()> {
    let packages: Vec<_> = opts
        .packages
        .iter()
        .map(|path| PathBuf::from(path))
        .collect();

    let (graph, pm) = crate::load::load(packages)?;
    match graph.order() {
        Err(err) => {
            tl_error!(
                "{$red}Circular dependency detected for:{/$} '{[green]}'",
                err.path().display()
            );
            return Err(());
        }
        Ok(order) => {
            order
                .map(|pd| process(pd, &pm, &opts))
                .collect::<Result<Vec<_>, _>>()?;
        }
    }

    Ok(())
}

#[inline]
fn process(pd: &PackageData, pm: &HashMap<PathBuf, CtxPath>, opts: &Options) -> Result<(), ()> {
    let ctxpath = pm.get(&pd.path).unwrap();
    tl_info!("Processing '{[green]}'", ctxpath.rel().display());

    Ok(())
}
