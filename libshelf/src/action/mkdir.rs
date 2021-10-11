use std::path::PathBuf;

use super::Resolve;

#[derive(Debug, Clone)]
pub struct MkdirAction {
    path: PathBuf,
}

impl Resolve for MkdirAction {
    #[inline]
    fn resolve(self, opts: &super::ResolveOpts) -> super::ResolveResult {
        todo!()
    }
}
