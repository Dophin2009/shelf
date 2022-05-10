use shelflib::{
    action::{
        generated::{self, Res},
        JsonAction, Resolve, TomlAction, YamlAction,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_yaml(
        &self,
        action: YamlAction,
        _path: &CtxPath,
    ) -> Result<Vec<Op<'static>>, ()> {
        let res = match action.resolve() {
            Ok(res) => res,
            Err(_err) => {
                // TODO: Output
                return Err(());
            }
        };

        handle_res(res)
    }

    #[inline]
    pub fn resolve_toml(
        &self,
        action: TomlAction,
        _path: &CtxPath,
    ) -> Result<Vec<Op<'static>>, ()> {
        let res = match action.resolve() {
            Ok(res) => res,
            Err(_err) => {
                // TODO: Output
                return Err(());
            }
        };

        handle_res(res)
    }

    #[inline]
    pub fn resolve_json(
        &self,
        action: JsonAction,
        _path: &CtxPath,
    ) -> Result<Vec<Op<'static>>, ()> {
        let res = match action.resolve() {
            Ok(res) => res,
            Err(_err) => {
                // TODO: Output
                return Err(());
            }
        };

        handle_res(res)
    }
}

#[inline]
fn handle_res(res: Res) -> Result<Vec<Op<'static>>, ()> {
    match res {
        Res::Normal(ops) => {
            // TODO: Output
            Ok(map_ops(ops))
        }
        Res::OverwriteContents(ops) => {
            // TODO: Output
            Ok(map_ops(ops))
        }
        Res::OverwriteFile(ops) => {
            // TODO: Output
            Ok(map_ops(ops))
        }
        Res::Skip(_skip) => {
            // TODO: Output
            Ok(vec![])
        }
    }
}

#[inline]
fn map_ops(ops: Vec<generated::Op>) -> Vec<Op<'static>> {
    ops.into_iter()
        .map(|op| match op {
            generated::Op::Rm(op) => Op::Rm(op),
            generated::Op::Create(op) => Op::Create(op),
            generated::Op::Write(op) => Op::Write(op),
            generated::Op::Mkdir(op) => Op::Mkdir(op),
        })
        .collect()
}

mod output {
    use std::path::Path;

    use shelflib::action::{JsonAction, TomlAction, YamlAction};

    use super::super::{describe, Describe, DescribeMode};
    use crate::ctxpath::CtxPath;
    use crate::output::{comb::sjoin4, Pretty};

    impl Describe for YamlAction {
        #[inline]
        fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
            common_describe("yaml", &self.dest, dest, mode)
        }
    }

    impl Describe for TomlAction {
        #[inline]
        fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
            common_describe("json", &self.dest, dest, mode)
        }
    }

    impl Describe for JsonAction {
        #[inline]
        fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
            common_describe("json", &self.dest, dest, mode)
        }
    }

    #[inline]
    fn common_describe(
        format: &str,
        action_dest: &Path,
        dest: &Path,
        mode: DescribeMode,
    ) -> Pretty {
        let action_dest = describe::dest_relative(action_dest, dest);
        sjoin4(
            "writing",
            format,
            "to",
            describe::mode_spath(action_dest, mode),
        )
    }
}
