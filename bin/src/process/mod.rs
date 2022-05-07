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
    op::{journal::OpJournal, Op},
};

use crate::ctxpath::CtxPath;

#[derive(Debug, Clone)]
pub struct ProcessorOptions {
    pub noop: bool,
    pub dest: PathBuf,
}

#[derive(Debug)]
pub struct Processor {
    opts: ProcessorOptions,
    journal: OpJournal,
}

#[derive(Debug)]
pub(self) struct GraphProcessor<'p, 'g> {
    opts: &'p ProcessorOptions,
    journal: &'p OpJournal,
    graph: &'g PackageGraph,
    paths: &'g HashMap<PathBuf, CtxPath>,
}

impl Processor {
    #[inline]
    pub fn new(opts: ProcessorOptions) -> Self {
        Self {
            opts,
            journal: OpJournal::new(),
        }
    }

    #[inline]
    pub fn process(
        &mut self,
        graph: &PackageGraph,
        paths: &HashMap<PathBuf, CtxPath>,
    ) -> Result<(), ()> {
        let mut processor = GraphProcessor::new(&self.opts, &self.journal, graph, paths);
        processor.process()
    }
}

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn new(
        opts: &'p ProcessorOptions,
        journal: &'p OpJournal,
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
    pub fn process_op(&mut self, _op: Op) -> Result<(), ()> {
        // TODO: Implement

        Ok(())
    }
}
