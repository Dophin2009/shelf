use shelflib::{
    action::{
        mkdir::{self, Res},
        MkdirAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_mkdir(&self, action: MkdirAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
        let res = action.resolve();
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
fn map_ops(ops: Vec<mkdir::Op>) -> Vec<Op<'static>> {
    ops.into_iter()
        .map(|op| match op {
            mkdir::Op::Rm(op) => Op::Rm(op),
            mkdir::Op::Mkdir(op) => Op::Mkdir(op),
        })
        .collect()
}
