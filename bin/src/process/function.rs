use shelflib::{
    action::{
        function::{self, Res},
        FunctionAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_function<'lua>(
        &self,
        action: FunctionAction<'lua>,
        _path: &CtxPath,
    ) -> Result<Vec<Op<'lua>>, ()> {
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
fn map_ops(ops: Vec<function::Op<'_>>) -> Vec<Op<'_>> {
    ops.into_iter()
        .map(|op| match op {
            function::Op::Function(op) => Op::Function(op),
        })
        .collect()
}

mod output {
    use std::path::Path;

    use shelflib::action::FunctionAction;

    use super::super::{Describe, DescribeMode};
    use crate::ctxpath::CtxPath;
    use crate::output::{comb::pretty, Pretty};

    impl<'lua> Describe for FunctionAction<'lua> {
        #[inline]
        fn describe(&self, _path: &CtxPath, _dest: &Path, _mode: DescribeMode) -> Pretty {
            pretty("calling lua function")
        }
    }
}
