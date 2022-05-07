mod command;
mod function;
mod generated;
mod link;
mod mkdir;
mod template;
mod tree;
mod write;

use std::collections::HashMap;
use std::path::PathBuf;

use shelflib::{
    action::Action,
    graph::{PackageData, PackageGraph},
    op::{
        ctx::FinishCtx,
        journal::{JournalOp, OpJournal},
        Finish, Op,
    },
};

use crate::ctxpath::CtxPath;

#[derive(Debug, Clone)]
pub struct ProcessorOptions {
    pub noop: bool,
    pub dest: PathBuf,

    pub ctx: FinishCtx,
}

#[derive(Debug)]
pub struct Processor<'j> {
    opts: ProcessorOptions,
    journal: &'j mut OpJournal,
}

#[derive(Debug)]
pub(self) struct GraphProcessor<'p, 'g> {
    opts: &'p ProcessorOptions,
    journal: &'p mut OpJournal,

    graph: &'g PackageGraph,
    paths: &'g HashMap<PathBuf, CtxPath>,
}

impl<'j> Processor<'j> {
    #[inline]
    pub fn new(opts: ProcessorOptions, journal: &'j mut OpJournal) -> Self {
        Self { opts, journal }
    }

    #[inline]
    pub fn process(
        &mut self,
        graph: &PackageGraph,
        paths: &HashMap<PathBuf, CtxPath>,
    ) -> Result<(), ()> {
        let mut processor = GraphProcessor::new(&self.opts, self.journal, graph, paths);
        processor.process()
    }
}

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn new(
        opts: &'p ProcessorOptions,
        journal: &'p mut OpJournal,
        graph: &'g PackageGraph,
        paths: &'g HashMap<PathBuf, CtxPath>,
    ) -> Self {
        Self {
            opts,
            journal,
            graph,
            paths,
        }
    }

    #[inline]
    pub fn process(&mut self) -> Result<(), ()> {
        match self.graph.order() {
            Ok(order) => {
                order
                    .map(|pd| self.process_package(pd))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(())
            }
            Err(_err) => {
                // TODO: Output
                Err(())
            }
        }
    }

    #[inline]
    pub fn process_package(&mut self, pd: &PackageData) -> Result<(), ()> {
        // SAFETY: Path guaranteed to be in it by `load`.
        let path = self.paths.get(&pd.path).unwrap();

        // output::info_processing(path);

        let aiter = pd.action_iter(&self.opts.dest);
        aiter
            .map(|action| self.process_action(action, path))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    #[inline]
    pub fn process_action(&mut self, action: Action, path: &CtxPath) -> Result<(), ()> {
        let ops = match action {
            Action::Link(action) => self.resolve_link(action, path),
            Action::Write(action) => self.resolve_write(action, path),
            Action::Mkdir(action) => self.resolve_mkdir(action, path),
            Action::Tree(action) => self.resolve_tree(action, path),
            Action::Handlebars(action) => self.resolve_handlebars(action, path),
            Action::Liquid(action) => self.resolve_liquid(action, path),
            Action::Yaml(action) => self.resolve_yaml(action, path),
            Action::Toml(action) => self.resolve_toml(action, path),
            Action::Json(action) => self.resolve_json(action, path),
            Action::Command(action) => self.resolve_command(action, path),
            Action::Function(action) => self.resolve_function(action, path),
        }?;

        ops.into_iter()
            .map(|op| self.process_op(op))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    #[inline]
    pub fn process_op(&mut self, op: Op) -> Result<(), ()> {
        match op {
            Op::Link(op) => self.process_journal_op(op),
            Op::LinkUndo(op) => self.process_journal_op(op),
            Op::Copy(op) => self.process_journal_op(op),
            Op::CopyUndo(op) => self.process_journal_op(op),
            Op::Create(op) => self.process_journal_op(op),
            Op::CreateUndo(op) => self.process_journal_op(op),
            Op::Write(op) => self.process_journal_op(op),
            Op::WriteUndo(op) => self.process_journal_op(op),
            Op::Mkdir(op) => self.process_journal_op(op),
            Op::MkdirUndo(op) => self.process_journal_op(op),
            Op::Rm(op) => self.process_journal_op(op),
            Op::RmUndo(op) => self.process_journal_op(op),
            Op::Command(op) => {
                // TODO: Output
                match op.finish(&self.opts.ctx) {
                    Ok(_fin) => {
                        // TODO: Output
                        Ok(())
                    }
                    Err(_err) => {
                        // TODO: Output
                        Err(())
                    }
                }
            }
            Op::Function(op) => {
                // TODO: Output
                match op.finish(&self.opts.ctx) {
                    Ok(_fin) => {
                        // TODO: Output
                        Ok(())
                    }
                    Err(_err) => {
                        // TODO: Output
                        Err(())
                    }
                }
            }
        }
    }

    #[inline]
    pub fn process_journal_op<O>(&mut self, op: O) -> Result<(), ()>
    where
        O: Into<JournalOp>,
    {
        {
            let mut t = self.journal.lock();
            match t.append_finish(op, &self.opts.ctx) {
                Ok(_fin) => {
                    // TODO: Output
                }
                Err(_err) => {
                    // TODO: Output
                    return Err(());
                }
            }
        }

        Ok(())
    }
}
