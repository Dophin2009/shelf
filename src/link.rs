use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use anyhow::Result;
use console::style;
use mlua::{Function, Lua};
use path_clean::PathClean;

use crate::action::{
    Action, CommandAction, FunctionAction, HandlebarsAction, JsonAction, LinkAction, LiquidAction,
    TomlAction, TreeAction, WriteAction, YamlAction,
};
use crate::format::Indexed;
use crate::graph::{OrderError, PackageGraph};
use crate::spec::{
    CmdHook, Directive, File, FunHook, GeneratedFile, GeneratedFileTyp, Hook, LinkType,
    RegularFile, Spec, TemplatedFile, TemplatedFileType, TreeFile,
};

#[inline]
pub fn link<'p>(
    dest: impl AsRef<Path>,
    graph: &'p PackageGraph,
) -> Result<impl Iterator<Item = PackageIter<'p>>, OrderError> {
    let order = graph.order()?;
    let it = order.into_iter().map(move |package| {
        link_one(
            dest.as_ref().to_path_buf(),
            &package.lua,
            &package.path,
            &package.data,
        )
    });

    Ok(it)
}

#[inline]
pub fn link_one<'p>(
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
        idxl: Indexed::new(spec.directives.len()),
    }
}

pub struct PackageIter<'p> {
    dest: PathBuf,
    path: &'p PathBuf,
    lua: &'p Lua,

    directives: VecDeque<&'p Directive>,

    idxl: Indexed,
}

impl<'p> Iterator for PackageIter<'p> {
    type Item = Action<'p>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let drct = self.directives.pop_front()?;
        let action = self.convert(drct);
        self.idxl.incr();

        Some(action)
    }
}

impl<'p> PackageIter<'p> {
    #[inline]
    fn convert(&self, drct: &Directive) -> Action<'p> {
        match drct {
            Directive::File(f) => self.convert_file(f),
            Directive::Hook(h) => self.convert_hook(h),
        }
    }

    #[inline]
    fn convert_file(&self, f: &File) -> Action<'p> {
        match f {
            File::Regular(rf) => self.convert_regular(rf),
            File::Templated(tf) => self.convert_template(tf),
            File::Tree(tf) => self.convert_tree(tf),
            File::Generated(gf) => self.convert_generated(gf),
        }
    }

    #[inline]
    fn convert_regular(&self, rf: &RegularFile) -> Action<'p> {
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
        // Normalize dest (or use src if absent).
        let dest_full = self.join_dest(dest.as_ref().unwrap_or(src));

        // Determine copy flag.
        let copy = match link_type {
            LinkType::Link => false,
            LinkType::Copy => true,
        };

        Action::Link(LinkAction {
            src: src_full,
            dest: dest_full,
            copy,
            optional: *optional,
        })
    }

    #[inline]
    fn convert_template(&self, tf: &TemplatedFile) -> Action<'p> {
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
        // Normalize dest.
        let dest_full = self.join_dest(dest.clone());

        match typ {
            TemplatedFileType::Handlebars(hbs) => Action::Handlebars(HandlebarsAction {
                src: src_full,
                dest: dest_full,
                vars: vars.clone(),
                optional: *optional,
                partials: hbs.partials.clone(),
            }),
            TemplatedFileType::Liquid(_) => Action::Liquid(LiquidAction {
                src: src_full,
                dest: dest_full,
                vars: vars.clone(),
                optional: *optional,
            }),
        }
    }

    #[inline]
    fn convert_tree(&self, tf: &TreeFile) -> Action<'p> {
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
            dest.as_ref()
                .map(|dest| dest.display().to_string())
                .unwrap_or(".".to_string()),
        ));

        // Normalize src.
        let src_full = self.join_package(src);
        // Normalize dest.
        let dest_full = dest
            .as_ref()
            .map(|dest| self.join_dest(dest))
            .unwrap_or_else(|| self.dest.clone());

        // FIXME no clone
        let globs = globs.clone().unwrap_or(vec!["**/*".to_string()]);
        let ignore = ignore.clone().unwrap_or(vec![]);

        // Determine copy flag.
        let copy = match link_type {
            LinkType::Link => false,
            LinkType::Copy => true,
        };

        Action::Tree(TreeAction {
            src: src_full,
            dest: dest_full,
            globs,
            ignore,
            copy,
            optional: *optional,
        })
    }

    #[inline]
    fn convert_generated(&self, gf: &GeneratedFile) -> Action<'p> {
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

        match typ {
            GeneratedFileTyp::Empty(_) => Action::Write(WriteAction {
                dest: dest_full,
                contents: "".to_string(),
            }),
            GeneratedFileTyp::String(s) => Action::Write(WriteAction {
                dest: dest_full,
                contents: s.contents.clone(),
            }),
            // FIXME error context
            GeneratedFileTyp::Yaml(y) => Action::Yaml(YamlAction {
                dest: dest_full,
                values: y.values.clone(),
                header: y.header.clone(),
            }),
            GeneratedFileTyp::Toml(t) => Action::Toml(TomlAction {
                dest: dest_full,
                values: t.values.clone(),
                header: t.header.clone(),
            }),
            GeneratedFileTyp::Json(j) => Action::Json(JsonAction {
                dest: dest_full,
                values: j.values.clone(),
            }),
        }
    }

    #[inline]
    fn convert_hook(&self, h: &Hook) -> Action<'p> {
        match h {
            Hook::Cmd(cmd) => self.convert_hook_cmd(cmd),
            Hook::Fun(fun) => self.convert_hook_fun(fun),
        }
    }

    #[inline]
    fn convert_hook_cmd(&self, cmd: &CmdHook) -> Action<'p> {
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

        Action::Command(CommandAction {
            command: command.clone(),
            quiet: quiet.unwrap_or(false),
            start: start_full,
            shell: shell.to_string(),
        })
    }

    #[inline]
    fn convert_hook_fun(&self, fun: &FunHook) -> Action<'p> {
        let FunHook { name, quiet } = fun;

        self.log_processing(&format!(
            "{} ({} {})",
            style("hook").bold().blue(),
            style("fn").bright(),
            style("<function>").italic().dim()
        ));

        // Load function from Lua registry.
        let function: Function = self.lua.named_registry_value(name).unwrap();

        Action::Function(FunctionAction {
            function,
            quiet: *quiet.as_ref().unwrap_or(&false),
        })
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
        self.idxl.debug(&format!("Processing: {}", step));
    }
}
