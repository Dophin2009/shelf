use std::collections::VecDeque;
use std::iter;
use std::path::{Path, PathBuf};

use anyhow::Result;
use console::style;
use log::debug;
use mlua::Lua;
use path_clean::PathClean;

use crate::action::{Action, LinkFileAction, WriteFileAction};
use crate::format::{Indexed, Sublevel};
use crate::graph::PackageGraph;
use crate::spec::{
    Directive, File, GeneratedFile, GeneratedFileTyp, LinkType, RegularFile, Spec, TemplatedFile,
    TemplatedFileType, TreeFile,
};
use crate::templating;

#[derive(Debug, Clone)]
pub struct Linker {
    dest: PathBuf,
}

impl Linker {
    #[inline]
    pub fn new(dest: impl AsRef<Path>) -> Self {
        Self {
            dest: dest.as_ref().to_path_buf().clean(),
        }
    }

    #[inline]
    pub fn link<'p>(
        &self,
        graph: &'p PackageGraph,
    ) -> Result<impl Iterator<Item = Result<Action<'p>>>> {
        // Link in dependency order.
        let order = graph.order()?;
        let dest = self.dest.clone();
        let actions = order.flat_map(move |package| {
            Self::link_one(dest.clone(), &package.lua, &package.path, &package.data)
        });

        Ok(actions)
    }

    #[inline]
    fn link_one<'p>(
        dest: PathBuf,
        lua: &'p Lua,
        path: &'p PathBuf,
        spec: &'p Spec,
    ) -> PackageIter<'p> {
        PackageIter {
            path,
            dest,
            lua,
            directives: spec.directives.iter().collect(),
            next: Box::new(iter::empty()),
            logger: Indexed::new(spec.directives.len()),
        }
    }
}

pub struct PackageIter<'p> {
    dest: PathBuf,
    path: &'p PathBuf,
    lua: &'p Lua,

    directives: VecDeque<&'p Directive>,
    next: Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>,

    logger: Indexed,
}

impl<'p> Iterator for PackageIter<'p> {
    type Item = Result<Action<'p>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.next.next() {
            item @ Some(_) => {
                return item;
            }
            None => {}
        }

        let drct = self.directives.pop_front()?;
        let it = self.convert(drct);
        self.next = Box::new(it);

        self.logger.incr();

        self.next()
    }
}

impl<'p> PackageIter<'p> {
    #[inline]
    fn convert(&self, drct: &Directive) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        match drct {
            Directive::File(f) => self.convert_file(f),
            Directive::Hook(_) => todo!(),
        }
    }

    #[inline]
    fn convert_file(&self, f: &File) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        match f {
            File::Regular(rf) => self.convert_regular(rf),
            File::Templated(tf) => Box::new(self.convert_template(tf)),
            File::Tree(tf) => Box::new(self.convert_tree(tf)),
            File::Generated(gf) => Box::new(self.convert_generated(gf)),
        }
    }

    #[inline]
    fn convert_regular(
        &self,
        rf: &RegularFile,
    ) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        let RegularFile {
            src,
            dest,
            link_type,
            optional,
        } = rf;

        self.log_processing(&format!(
            "{} ({} {} {} {})",
            style("file").bold().cyan(),
            match &link_type {
                LinkType::Link => style("link").green(),
                LinkType::Copy => style("copy").yellow(),
            },
            src.display(),
            style("->").dim(),
            dest.as_ref().unwrap_or(&src).display()
        ));

        // Normalize src.
        let src_full = self.join_package(src);
        // If optional flag enabled, and src doesn't exist, skip.
        if *optional && !src_full.exists() {
            self.log_skipping(&format!("{} does not exist...", src.display()));
            return Box::new(iter::empty());
        }

        // Normalize dest (or use src if absent).
        let dest_full = self.join_dest(dest.as_ref().unwrap_or(src));

        // Determine copy flag.
        let copy = match link_type {
            LinkType::Link => false,
            LinkType::Copy => true,
        };

        let it = iter::once(Ok(Action::LinkFile(LinkFileAction {
            src: src_full,
            dest: dest_full,
            copy,
        })));
        Box::new(it)
    }

    #[inline]
    fn convert_template(
        &self,
        tf: &TemplatedFile,
    ) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        let TemplatedFile {
            src,
            dest,
            vars,
            typ,
            optional,
        } = tf;

        self.log_processing(&format!(
            "{} ({} {} {} {})",
            style("template").bold().yellow(),
            match typ {
                TemplatedFileType::Handlebars(_) => style("hbs").red(),
                TemplatedFileType::Liquid(_) => style("liquid").blue(),
            },
            src.display(),
            style("->").dim(),
            dest.display()
        ));

        // Normalize src.
        let src_full = self.join_package(&src);

        // If optional flag enabled, and file does not exist, skip.
        if *optional && !src_full.exists() {
            self.log_skipping(&format!("{} does not exist...", src.display()));
            return Box::new(iter::empty());
        }

        // Normalize dest.
        let dest_full = self.join_dest(dest.clone());

        // Generate template contents.
        let contents = match &typ {
            TemplatedFileType::Handlebars(hbs) => {
                templating::hbs::render(&src_full, &vars, &hbs.partials)
            }
            TemplatedFileType::Liquid(lq) => templating::liquid::render(&src_full, &vars),
        };

        let it = iter::once_with(|| {
            Ok(Action::WriteFile(WriteFileAction {
                dest: dest_full,
                contents: contents?,
            }))
        });
        Box::new(it)
    }

    #[inline]
    fn convert_tree(&self, tf: &TreeFile) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        let dest = tf
            .dest
            .as_ref()
            .map(|dest| self.join_dest(dest))
            .unwrap_or_else(|| self.dest.clone());

        Box::new(iter::empty())
    }

    #[inline]
    fn convert_generated(
        &self,
        gf: &GeneratedFile,
    ) -> Box<dyn Iterator<Item = Result<Action<'p>>> + 'p> {
        let dest = self.dest.join(&gf.dest);
        let contents = match &gf.typ {
            GeneratedFileTyp::Empty(_) => "".to_string(),
            GeneratedFileTyp::String(s) => s.contents.clone(),
            GeneratedFileTyp::Yaml(_) => todo!(),
            GeneratedFileTyp::Toml(_) => todo!(),
            GeneratedFileTyp::Json(_) => todo!(),
        };

        let it = iter::once(Ok(Action::WriteFile(WriteFileAction { dest, contents })));
        Box::new(it)
    }

    #[inline]
    fn join_package(&self, path: impl AsRef<Path>) -> PathBuf {
        self.normalize_path(path, &self.path)
    }

    #[inline]
    fn join_dest(&self, path: impl AsRef<Path>) -> PathBuf {
        self.normalize_path(path, &self.dest)
    }

    #[inline]
    fn normalize_path(&self, path: impl AsRef<Path>, start: &PathBuf) -> PathBuf {
        let new_path = if path.as_ref().is_relative() {
            start.join(path)
        } else {
            path.as_ref().to_path_buf()
        };
        new_path.clean()
    }

    #[inline]
    fn log_processing(&self, step: &str) {
        self.logger.debug(&format!("Processing: {}", step));
    }

    #[inline]
    fn log_skipping(&self, reason: &str) {
        Sublevel::default().debug(&format!("{} {}", style("Skipping:").bold(), reason));
    }
}
