mod link;

use std::collections::HashMap;
use std::path::PathBuf;

use shelflib::{
    action::{Action, Resolve},
    graph::{PackageData, PackageGraph},
};

use crate::ctxpath::CtxPath;

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub noop: bool,
    pub dest: PathBuf,
}

#[inline]
pub fn process(
    graph: &PackageGraph,
    pm: &HashMap<PathBuf, CtxPath>,
    opts: ProcessOptions,
) -> Result<(), ()> {
    match graph.order() {
        Err(err) => {
            tl_error!(
                "{$red+bold}fatal:{/$} circular dependency detected for: '{[green]}'",
                err.path().display()
            );
            Err(())
        }
        Ok(order) => {
            order
                .map(|pd| process_one(pd, &pm, &opts))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(())
        }
    }
}

#[inline]
fn process_one(
    pd: &PackageData,
    pm: &HashMap<PathBuf, CtxPath>,
    opts: &ProcessOptions,
) -> Result<(), ()> {
    // SAFETY: Path guaranteed to be in it by `load`.
    let path = pm.get(&pd.path).unwrap();
    tl_info!("Processing '{[green]}'", path.rel().display());

    let aiter = pd.action_iter(&opts.dest);
    aiter
        .map(|action| match action {
            Action::Link(action) => link::process(action, path, &opts.dest),
            _ => Ok(()),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(())
}
