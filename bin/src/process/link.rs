use shelflib::{
    action::{
        link::{self, Error, Res},
        LinkAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_link(&self, action: LinkAction, path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
        output::processing_link(&action, path, &self.opts.dest);

        let res = match action.resolve() {
            Ok(res) => res,
            Err(err) => {
                match err {
                    Error::SrcMissing => output::src_missing(&action, path, &self.opts.dest),
                }

                return Err(());
            }
        };

        match res {
            Res::Normal(ops) => Ok(map_ops(ops)),
            Res::Overwrite(ops) => {
                output::overwriting(&action, path, &self.opts.dest);
                Ok(map_ops(ops))
            }
            Res::Skip(skip) => {
                output::skipping(&skip, &action, path, &self.opts.dest);
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

mod output {
    use std::path::Path;

    use shelflib::action::{link::Skip, LinkAction};

    use super::super::{describe, Describe, DescribeMode};
    use crate::ctxpath::CtxPath;
    use crate::output::{
        comb::{sjoin2, sjoin4},
        Pretty, Step,
    };

    impl Describe for LinkAction {
        #[inline]
        fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
            let src = describe::path_relative(&self.src, path);
            let dest = describe::dest_relative(&self.dest, dest);
            sjoin4(
                "linking",
                describe::mode_spath(src, mode),
                "to",
                describe::mode_spath(dest, mode),
            )
        }
    }

    #[inline]
    pub fn processing_link(action: &LinkAction, path: &CtxPath, dest: &Path) {
        Step::message(action.describe_info(path, dest));
    }

    #[inline]
    pub fn src_missing(action: &LinkAction, path: &CtxPath, dest: &Path) {
        Step::error().message(sjoin2(
            "missing source",
            describe::sdest_relative(&action.src, dest),
        ));
        Step::error().context(action.describe_info(path, dest));
    }

    #[inline]
    pub fn overwriting(action: &LinkAction, path: &CtxPath, dest: &Path) {
        Step::warning().message(sjoin2(
            "overwriting existing",
            describe::sdest_relative(&action.dest, dest),
        ));
        Step::warning().context(action.describe_info(path, dest));
    }

    #[inline]
    pub fn skipping(skip: &Skip, action: &LinkAction, path: &CtxPath, dest: &Path) {
        let message = match skip {
            Skip::SameSrcDest => sjoin2(
                "same source and destination",
                describe::sdest_relative(&action.src, dest),
            ),
            Skip::OptMissing => sjoin2(
                "missing optional source",
                describe::sdest_relative(&action.src, dest),
            ),
            Skip::DestExists => sjoin2(
                "existing destination",
                describe::sdest_relative(&action.dest, dest),
            ),
        };

        Step::skipping().message(message);
        Step::skipping().context(action.describe_info(path, dest));
    }
}
