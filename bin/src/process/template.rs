use shelflib::{
    action::{
        template::{self, Res},
        HandlebarsAction, LiquidAction, Resolve,
    },
    op::Op,
};

use super::GraphProcessor;
use crate::ctxpath::CtxPath;

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn resolve_handlebars(
        &self,
        action: HandlebarsAction,
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
    pub fn resolve_liquid(
        &self,
        action: LiquidAction,
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
fn map_ops(ops: Vec<template::Op>) -> Vec<Op<'static>> {
    ops.into_iter()
        .map(|op| match op {
            template::Op::Rm(op) => Op::Rm(op),
            template::Op::Create(op) => Op::Create(op),
            template::Op::Write(op) => Op::Write(op),
            template::Op::Mkdir(op) => Op::Mkdir(op),
        })
        .collect()
}
