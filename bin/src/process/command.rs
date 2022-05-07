
use shelflib::{
    action::{
        command::{self, Res},
        CommandAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_command(
        &self,
        action: CommandAction,
        _path: &CtxPath,
    ) -> Result<Vec<Op<'static>>, ()> {
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
        }
    }
}

#[inline]
fn map_ops(ops: Vec<command::Op>) -> Vec<Op<'static>> {
    ops.into_iter()
        .map(|op| match op {
            command::Op::Command(op) => Op::Command(op),
        })
        .collect()
}
