use shelflib::{
    action::{link, tree::Res, Resolve, TreeAction},
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_tree(&self, action: TreeAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
        let res = match action.resolve() {
            Ok(res) => res,
            Err(_err) => {
                // TODO: Output
                return Err(());
            }
        };

        match res {
            Res::Normal(res) => {
                // TODO: Output
                let ops = res
                    .into_iter()
                    .flat_map(|res| match res {
                        link::Res::Normal(ops) => super::link::map_ops(ops),
                        link::Res::Overwrite(ops) => super::link::map_ops(ops),
                        link::Res::Skip(_skip) => {
                            // TODO: Output
                            vec![]
                        }
                    })
                    .collect();
                Ok(ops)
            }
            Res::Skip(_skip) => {
                // TODO: Output
                Ok(vec![])
            }
        }
    }
}
