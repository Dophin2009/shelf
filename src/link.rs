use std::collections::VecDeque;
use std::iter;
use std::path::{Path, PathBuf};

use anyhow::Result;
use mlua::Lua;
use path_clean::PathClean;

use crate::action::{Action, LinkFileAction, WriteFileAction};
use crate::graph::PackageGraph;
use crate::spec::{
    Directive, File, GeneratedFile, GeneratedFileTyp, LinkType, Spec, TemplatedFile,
    TemplatedFileType, TreeFile,
};

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
    pub fn link<'p>(&self, graph: &'p PackageGraph) -> Result<impl Iterator<Item = Action<'p>>> {
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
        }
    }
}

pub struct PackageIter<'p> {
    dest: PathBuf,
    path: &'p PathBuf,
    lua: &'p Lua,

    directives: VecDeque<&'p Directive>,
    next: Box<dyn Iterator<Item = Action<'p>> + 'p>,
}

impl<'p> Iterator for PackageIter<'p> {
    type Item = Action<'p>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.next.next() {
            item @ Some(_) => {
                return item;
            }
            None => {}
        }

        let drct = self.directives.pop_front()?;
        let it = self.convert_directive(drct);
        self.next = Box::new(it);

        self.next()
    }
}

impl<'p> PackageIter<'p> {
    #[inline]
    fn convert_directive(&self, drct: &Directive) -> Box<dyn Iterator<Item = Action<'p>> + 'p> {
        match drct {
            Directive::File(f) => self.convert_file_directive(f),
            Directive::Hook(_) => todo!(),
        }
    }

    #[inline]
    fn convert_file_directive(&self, f: &File) -> Box<dyn Iterator<Item = Action<'p>> + 'p> {
        match f {
            File::Regular(rf) => {
                let src = self.join_package(&rf.src);
                let dest = rf
                    .dest
                    .as_ref()
                    .map(|dest| self.join_dest(dest))
                    .unwrap_or_else(|| src.clone());
                let copy = match rf.link_type {
                    LinkType::Link => false,
                    LinkType::Copy => true,
                };

                Box::new(Some(Action::LinkFile(LinkFileAction { src, dest, copy })).into_iter())
            }
            File::Templated(tf) => Box::new(self.convert_template_directive(tf)),
            File::Tree(tf) => Box::new(self.convert_tree_directive(tf)),
            File::Generated(gf) => Box::new(self.convert_generated_directive(gf)),
        }
    }

    #[inline]
    fn convert_template_directive(
        &self,
        tf: &TemplatedFile,
    ) -> Box<dyn Iterator<Item = Action<'p>> + 'p> {
        let dest = self.join_dest(tf.dest.clone());
        let contents = match tf.typ {
            TemplatedFileType::Handlebars(_) => "",
            TemplatedFileType::Liquid(_) => "",
        }
        .to_string();

        let it = iter::once(Action::WriteFile(WriteFileAction { dest, contents }));
        Box::new(it)
    }

    #[inline]
    fn convert_tree_directive(&self, tf: &TreeFile) -> Box<dyn Iterator<Item = Action<'p>> + 'p> {
        let dest = tf
            .dest
            .as_ref()
            .map(|dest| self.join_dest(dest))
            .unwrap_or_else(|| self.dest.clone());

        Box::new(iter::empty())
    }

    #[inline]
    fn convert_generated_directive(
        &self,
        gf: &GeneratedFile,
    ) -> Box<dyn Iterator<Item = Action<'p>> + 'p> {
        let dest = self.dest.join(&gf.dest);
        let contents = match &gf.typ {
            GeneratedFileTyp::Empty(_) => "".to_string(),
            GeneratedFileTyp::String(s) => s.contents.clone(),
            GeneratedFileTyp::Yaml(_) => todo!(),
            GeneratedFileTyp::Toml(_) => todo!(),
            GeneratedFileTyp::Json(_) => todo!(),
        };

        let it = iter::once(Action::WriteFile(WriteFileAction { dest, contents }));
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
        if path.as_ref().is_relative() {
            start.join(path)
        } else {
            path.as_ref().to_path_buf()
        }
    }
}
