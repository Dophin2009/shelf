use std::borrow::Cow;
use std::collections::{HashSet, VecDeque};
use std::iter;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use console::style;
use log::debug;
use mlua::{Function, Lua};
use path_clean::PathClean;

use crate::action::{Action, LinkFileAction, RunCommandAction, RunFunctionAction, WriteFileAction};
use crate::format::{Indexed, Sublevel};
use crate::graph::PackageGraph;
use crate::spec::{
    CmdHook, Directive, File, FunHook, GeneratedFile, GeneratedFileTyp, Hook, LinkType,
    RegularFile, Spec, TemplatedFile, TemplatedFileType, TreeFile,
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
        let it = match self.convert(drct) {
            Ok(it) => it,
            Err(err) => return Some(Err(err)),
        };
        self.next = it;

        self.logger.incr();

        self.next()
    }
}

impl<'p> PackageIter<'p> {
    #[inline]
    fn convert(
        &self,
        drct: &Directive,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        match drct {
            Directive::File(f) => self.convert_file(f),
            Directive::Hook(h) => self.convert_hook(h),
        }
    }

    #[inline]
    fn convert_file(&self, f: &File) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        match f {
            File::Regular(rf) => self.convert_regular(rf),
            File::Templated(tf) => self.convert_template(tf),
            File::Tree(tf) => self.convert_tree(tf),
            File::Generated(gf) => self.convert_generated(gf),
        }
    }

    #[inline]
    fn convert_regular(
        &self,
        rf: &RegularFile,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
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
            self.log_skipping(&format!(
                "{} does not exist",
                style(src.display()).underlined()
            ));
            return Ok(Box::new(iter::empty()));
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
        Ok(Box::new(it))
    }

    #[inline]
    fn convert_template(
        &self,
        tf: &TemplatedFile,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
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
            self.log_skipping(&format!(
                "{} does not exist",
                style(src.display()).underlined()
            ));
            return Ok(Box::new(iter::empty()));
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
        Ok(Box::new(it))
    }

    #[inline]
    fn convert_tree(
        &self,
        tf: &TreeFile,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        let TreeFile {
            src,
            dest,
            globs,
            ignore,
            link_type,
            optional,
        } = tf;

        self.log_processing(&format!(
            "{} ({} {} {} {})",
            style("tree").bold().green(),
            match link_type {
                LinkType::Link => style("link").green(),
                LinkType::Copy => style("copy").yellow(),
            },
            src.display(),
            style("->").dim(),
            dest.as_ref().unwrap_or(src).display(),
        ));

        // Normalize src.
        let src_full = self.join_package(src);

        // If optional flag enabled, and src does not exist, skip.
        if !src_full.exists() {
            if *optional {
                self.log_skipping(&format!(
                    "{} does not exist",
                    style(src.display()).underlined()
                ));
                return Ok(Box::new(iter::empty()));
            } else {
                return Err(anyhow!("Tree path not found: {}", src.display()));
            }
        }

        // Normalize dest.
        let dest_full = dest
            .as_ref()
            .map(|dest| self.join_dest(dest))
            .unwrap_or_else(|| self.dest.clone());

        #[inline]
        fn glob_tree(src: impl AsRef<Path>, pats: &Vec<String>) -> Result<HashSet<PathBuf>> {
            let pats: Vec<_> = pats
                .iter()
                .map(|glob| format!("{}/{}", src.as_ref().display(), glob))
                .collect();
            let matches: Vec<glob::Paths> = pats
                .iter()
                .map(|pat| {
                    glob::glob(pat).with_context(|| format!("Couldn't glob with pattern: {}", pat))
                })
                .collect::<Result<_>>()?;
            matches
                .into_iter()
                .flatten()
                .map(|r| r.with_context(|| "Couldn't read path"))
                .collect::<Result<_>>()
        }

        // Glob to get file paths.
        // FIXME handle absolute path globs
        let globs = globs
            .as_ref()
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Owned(vec!["**/*".to_string()]));
        let mut paths = glob_tree(&src_full, &globs)?;

        let ignore = ignore
            .as_ref()
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Owned(vec![]));
        let ignore_paths = glob_tree(&src_full, &ignore)?;

        for path in ignore_paths {
            paths.remove(&path);
        }

        todo!()

        Ok(Box::new(iter::empty()))
    }

    #[inline]
    fn convert_generated(
        &self,
        gf: &GeneratedFile,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        let GeneratedFile { dest, typ } = gf;

        self.log_processing(&format!(
            "{} ({} {})",
            style("generate").bold().magenta(),
            match &typ {
                GeneratedFileTyp::Empty(_) => style("empty").white(),
                GeneratedFileTyp::String(_) => style("string").blue(),
                GeneratedFileTyp::Yaml(_) => style("yaml").green(),
                GeneratedFileTyp::Toml(_) => style("toml").yellow(),
                GeneratedFileTyp::Json(_) => style("json").red(),
            },
            dest.display()
        ));

        // Normalize dest.
        let dest_full = self.join_dest(&dest);

        // Generate file contents.
        let (header, mut contents) = match &typ {
            GeneratedFileTyp::Empty(_) => (None, Ok("".to_string())),
            GeneratedFileTyp::String(s) => (None, Ok(s.contents.clone())),
            // FIXME error context
            GeneratedFileTyp::Yaml(y) => {
                let contents = serde_yaml::to_string(&y.values)
                    .with_context(|| "Couldn't serialize values to yaml");
                (y.header.as_ref(), contents)
            }
            GeneratedFileTyp::Toml(t) => {
                let contents = toml::to_string_pretty(&t.values)
                    .with_context(|| "Couldn't serialize values to toml");
                (t.header.as_ref(), contents)
            }
            GeneratedFileTyp::Json(j) => (
                None,
                serde_json::to_string(&j.values)
                    .with_context(|| "Couldn't serialize values to json"),
            ),
        };

        // Prepend the header if there is one.
        contents = match header {
            Some(header) => contents.map(|contents| format!("{}\n{}", header, contents)),
            None => contents,
        };

        let it = iter::once_with(|| {
            Ok(Action::WriteFile(WriteFileAction {
                dest: dest_full,
                contents: contents?,
            }))
        });
        Ok(Box::new(it))
    }

    #[inline]
    fn convert_hook(&self, h: &Hook) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        match h {
            Hook::Cmd(cmd) => self.convert_hook_cmd(cmd),
            Hook::Fun(fun) => self.convert_hook_fun(fun),
        }
    }

    #[inline]
    fn convert_hook_cmd(
        &self,
        cmd: &CmdHook,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        let CmdHook {
            command,
            quiet,
            start,
            shell,
        } = cmd;

        // Use sh as default shell.
        let shell = shell.as_ref().map(String::as_str).unwrap_or("sh");
        self.log_processing(&format!(
            "{} ({} '{}')",
            style("hook").bold().blue(),
            style(shell).bright(),
            style(command).dim(),
        ));

        // Normalize start path.
        let start_full = start
            .as_ref()
            .map(|start| self.join_package(start))
            .unwrap_or(self.path.clone());

        let action = Action::RunCommand(RunCommandAction {
            command: command.clone(),
            quiet: quiet.unwrap_or(false),
            start: start_full,
            shell: shell.to_string(),
        });
        let it = iter::once(Ok(action));
        Ok(Box::new(it))
    }

    #[inline]
    fn convert_hook_fun(
        &self,
        fun: &FunHook,
    ) -> Result<Box<dyn Iterator<Item = Result<Action<'p>>> + 'p>> {
        let FunHook { name, quiet } = fun;

        self.log_processing(&format!(
            "{} ({} {})",
            style("hook").bold().blue(),
            style("fn").bright(),
            style("<function>").italic().dim()
        ));

        // Load function from Lua registry.
        let function: Function = self.lua.named_registry_value(&fun.name).unwrap();

        let action = Action::RunFunction(RunFunctionAction { function });
        let it = iter::once(Ok(action));
        Ok(Box::new(it))
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
        Sublevel::default().debug(&format!("{} {}", style("Skipping...").bold(), reason));
    }
}
