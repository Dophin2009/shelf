use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};
use mlua::Lua;
use path_clean::PathClean;

use crate::action::{Action, LinkFileAction, WriteFileAction};
use crate::graph::{PackageGraph, PackageState};
use crate::spec::{Directive, File, LinkType, RegularFile};
use crate::{
    GeneratedFile, GeneratedFileTyp, TemplatedFile, TemplatedFileType, TreeFile, Verbosity,
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

    // #[inline]
    // pub fn link<'p>(
    // &self,
    // graph: &'p PackageGraph,
    // ) -> Result<impl Iterator<Item = Result<Action<'p>>>> {
    // // Link in dependency order.
    // let order = graph.order()?;
    // let actions = order.flat_map(|package| self.link_one(&package));

    // Ok(actions)
    // }

    #[inline]
    fn link_one<'p>(&self, package: &'p PackageState) -> impl Iterator<Item = Action<'p>> {
        let actions = package
            .data
            .directives
            .into_iter()
            .map(|drct| self.convert_directive(&drct));

        Ok(actions)
    }

    #[inline]
    pub fn convert_directive<'p>(&self, drct: &Directive) -> impl Iterator<Item = Action<'p>> {
        match drct {
            Directive::File(f) => self.convert_file_directive(f),
            Directive::Hook(_) => todo!(),
        }
    }

    #[inline]
    pub fn convert_file_directive<'p>(&self, f: &File) -> Action<'p> {
        match f {
            File::Regular(rf) => {
                let src = rf.src;
                let dest = self.join_dest(tf.dest);
                let copy = match rf.link_type {
                    LinkType::Link => false,
                    LinkType::Copy => true,
                };

                Action::LinkFile(LinkFileAction { src, dest, copy })
            }
            File::Templated(tf) => self.convert_template_directive(tf),
            File::Tree(tf) => self.convert_tree_directive(tf),
            File::Generated(gf) => self.convert_generated_directive(gf),
        }
    }

    #[inline]
    pub fn convert_template_directive<'p>(&self, tf: &TemplatedFile) -> Action<'p> {
        let dest = self.join_dest(tf.dest);
        let contents = match tf.typ {
            TemplatedFileType::Handlebars(hbs) => todo!(),
            TemplatedFileType::Liquid(_) => todo!(),
        };

        Action::WriteFile(WriteFileAction { dest, contents })
    }

    #[inline]
    pub fn convert_tree_directive<'p>(&self, tf: &TreeFile) -> Action<'p> {
        let dest = self.join_dest(tf.dest);
    }

    #[inline]
    pub fn convert_generated_directive<'p>(&self, gf: &GeneratedFile) -> Action<'p> {
        let dest = self.dest.join(gf.dest);
        let contents = match gf.typ {
            GeneratedFileTyp::Empty(e) => "".to_string(),
            GeneratedFileTyp::String(s) => s.contents,
            GeneratedFileTyp::Yaml(_) => todo!(),
            GeneratedFileTyp::Toml(_) => todo!(),
            GeneratedFileTyp::Json(_) => todo!(),
        };

        Action::WriteFile(WriteFileAction { dest, contents })
    }

    #[inline]
    fn join_dest(&self, path: PathBuf) -> PathBuf {
        self.dest.join(path)
    }

    #[inline]
    fn normalize_path(&self, path: PathBuf, start: &PathBuf) -> PathBuf {
        if path.is_relative() {
            start.join(path)
        } else {
            path
        }
    }
}
