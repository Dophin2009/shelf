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
    pub fn resolve_yaml(&self, action: YamlAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
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
    pub fn resolve_toml(&self, action: TomlAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
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
    pub fn resolve_json(&self, action: JsonAction, _path: &CtxPath) -> Result<Vec<Op<'static>>, ()> {
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
