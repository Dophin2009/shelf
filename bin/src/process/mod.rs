mod command;
mod function;
mod generated;
mod link;
mod mkdir;
mod template;
mod tree;
mod write;

mod op;

mod describe;
mod output;

use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use shelflib::{
    action::Action,
    graph::{PackageData, PackageGraph},
    op::{ctx::FinishCtx, journal::OpJournal},
};

use crate::ctxpath::CtxPath;
use crate::output::Pretty;

pub(self) use self::describe::{Describe, DescribeMode};

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
}

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn process(&mut self) -> Result<(), ()> {
        match self.graph.order() {
            Ok(order) => {
                order
                    .map(|pd| self.process_package(pd))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(())
            }
            Err(err) => {
                output::error_circular(err);
                Err(())
            }
        }
    }

    #[inline]
    pub fn process_package(&mut self, pd: &PackageData) -> Result<(), ()> {
        // SAFETY: Path guaranteed to be in it by `load`.
        let path = self.paths.get(&pd.path).unwrap();

        output::processing(path);

        let aiter = pd.action_iter(&self.opts.dest);
        aiter
            .map(|action| self.process_action(action, path, &self.opts.dest))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    #[inline]
    pub fn process_action(
        &mut self,
        action: Action,
        path: &CtxPath,
        dest: &Path,
    ) -> Result<(), ()> {
        let ops = match action.clone() {
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
            .map(|op| self.process_op(&action, op, path, dest))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
}

impl<'lua> Describe for Action<'lua> {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        match self {
            Action::Link(action) => action.describe(path, dest, mode),
            Action::Write(action) => action.describe(path, dest, mode),
            Action::Tree(action) => action.describe(path, dest, mode),
            Action::Handlebars(action) => action.describe(path, dest, mode),
            Action::Liquid(action) => action.describe(path, dest, mode),
            Action::Yaml(action) => action.describe(path, dest, mode),
            Action::Toml(action) => action.describe(path, dest, mode),
            Action::Json(action) => action.describe(path, dest, mode),
            Action::Mkdir(action) => action.describe(path, dest, mode),
            Action::Command(action) => action.describe(path, dest, mode),
            Action::Function(action) => action.describe(path, dest, mode),
        }
    }
}
