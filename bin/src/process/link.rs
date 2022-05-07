use shelflib::{
    action::{
        link::{self, Res},
        LinkAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_link(&self, action: LinkAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
        let res = match action.resolve() {
            Ok(res) => res,
            Err(_err) => {
                // TODO: Output
                return Err(());
            }
        };

        match res {
            Res::Normal(ops) => {
                // TODO: Output
                Ok(map_ops(ops))
            }
            Res::Overwrite(ops) => {
                // TODO: Output
                Ok(map_ops(ops))
            }
            Res::Skip(_skip) => {
                // TODO: Output
                Ok(vec![])
            }
        }
    }
}

#[inline]
pub fn map_ops(ops: Vec<link::Op>) -> Vec<Op<'static>> {
    ops.into_iter()
        .map(|op| match op {
            link::Op::Rm(op) => Op::Rm(op),
            link::Op::Link(op) => Op::Link(op),
            link::Op::Copy(op) => Op::Copy(op),
            link::Op::Mkdir(op) => Op::Mkdir(op),
        })
        .collect()
}
