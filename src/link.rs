use std::collections::VecDeque;
use std::path::Path;

use anyhow::Result;
use mlua::{Function, Lua};

use crate::action::{
    Action, CommandAction, FunctionAction, HandlebarsAction, JsonAction, LinkAction, LiquidAction,
    TomlAction, TreeAction, WriteAction, YamlAction,
};
use crate::error::EmptyError;
use crate::graph::PackageGraph;
use crate::pathutil::PathWrapper;
use crate::spec::{
    CmdHook, Directive, EnvMap, File, FunHook, GeneratedFile, GeneratedFileTyp, Hook, LinkType,
    NonZeroExitBehavior, RegularFile, Spec, TemplatedFile, TemplatedFileType, TreeFile,
};

#[inline]
pub fn link<'d, 'p>(
    dest: &'d PathWrapper,
    graph: &'p PackageGraph,
) -> Result<impl Iterator<Item = PackageIter<'d, 'p>>, EmptyError> {
    let order = fail!(graph.order(), err => {
        sl_error!("{$red}Circular dependency found for package:{/$} {}", err.0.absd());
    });

    let it = order.into_iter().map(move |package| {
        link_one(
            package.data.name.clone(),
            dest,
            &package.lua,
            &package.path,
            &package.data,
        )
    });

    Ok(it)
}

#[inline]
fn link_one<'d, 'p>(
    name: String,
    dest: &'d PathWrapper,
    lua: &'p Lua,
    path: &'p PathWrapper,
    spec: &'p Spec,
) -> PackageIter<'d, 'p> {
    PackageIter {
        name,
        path,
        dest,
        lua,
        directives: spec.directives.iter().collect(),
        i: 0,
        n: spec.directives.len(),
    }
}

pub struct PackageIter<'d, 'p> {
    name: String,

    dest: &'d PathWrapper,
    path: &'p PathWrapper,
    lua: &'p Lua,

    directives: VecDeque<&'p Directive>,

    i: usize,
    n: usize,
}

impl<'d, 'p> Iterator for PackageIter<'d, 'p> {
    type Item = Action<'p>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let drct = self.directives.pop_front()?;
        let action = self.convert(drct);

        self.i += 1;

        Some(action)
    }
}

impl<'d, 'p> PackageIter<'d, 'p> {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

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

        let (link_s, copy_s) = match link_type {
            LinkType::Link => ("link", ""),
            LinkType::Copy => ("", "copy"),
        };
        idx_debug!(
            self.i,
            self.n,
            "{$cyan+bold}file{/$} ({[green]}{[yellow]} {[green]} {$dimmed}->{/$} {[green]})",
            link_s,
            copy_s,
            src.display(),
            dest.as_ref().unwrap_or(&src).display()
        );

        // Normalize src.
        let src_w = self.join_package(src);
        // Normalize dest (or use src if absent).
        let dest_w = self.join_dest(dest.as_ref().unwrap_or(src));

        // Determine copy flag.
        let copy = match link_type {
            LinkType::Link => false,
            LinkType::Copy => true,
        };

        Action::Link(LinkAction {
            src: src_w,
            dest: dest_w,
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

        let (hbs_s, liquid_s) = match typ {
            TemplatedFileType::Handlebars(_) => ("hbs", ""),
            TemplatedFileType::Liquid(_) => ("", "liquid"),
        };
        idx_debug!(
            self.i,
            self.n,
            "{$yellow+bold}template{/$} ({[red]}{[blue]} {[green]} {$dimmed}->{/$} {[green]})",
            hbs_s,
            liquid_s,
            src.display(),
            dest.display()
        );

        // Normalize src.
        let src_w = self.join_package(src);
        // Normalize dest.
        let dest_w = self.join_dest(dest);

        match typ {
            TemplatedFileType::Handlebars(hbs) => Action::Handlebars(HandlebarsAction {
                src: src_w,
                dest: dest_w,
                vars: vars.clone(),
                optional: *optional,
                partials: hbs.partials.clone(),
            }),
            TemplatedFileType::Liquid(_) => Action::Liquid(LiquidAction {
                src: src_w,
                dest: dest_w,
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

        let (link_s, copy_s) = match link_type {
            LinkType::Link => ("link", ""),
            LinkType::Copy => ("", "copy"),
        };
        idx_debug!(
            self.i,
            self.n,
            "{$yellow+bold}template{/$} ({[green]}{[yellow]} {[green]} {$dimmed}->{/$} {[green]})",
            link_s,
            copy_s,
            src.display(),
            dest.as_ref()
                .map(|dest| dest.display().to_string())
                .unwrap_or(".".to_string())
        );

        // Normalize src.
        let src_w = self.join_package(src);
        // Normalize dest.
        let dest_w = dest
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
            src: src_w,
            dest: dest_w,
            globs,
            ignore,
            copy,
            optional: *optional,
        })
    }

    #[inline]
    fn convert_generated(&self, gf: &GeneratedFile) -> Action<'p> {
        let GeneratedFile { dest, typ } = gf;

        // FIXME this is terrible
        let (empty_s, string_s, yaml_s, toml_s, json_s) = match &typ {
            GeneratedFileTyp::Empty(_) => ("empty", "", "", "", ""),
            GeneratedFileTyp::String(_) => ("", "toml", "", "", ""),
            GeneratedFileTyp::Yaml(_) => ("", "", "yaml", "", ""),
            GeneratedFileTyp::Toml(_) => ("", "", "", "toml", ""),
            GeneratedFileTyp::Json(_) => ("", "", "", "", "json"),
        };
        idx_debug!(
            self.i,
            self.n,
            "{$magenta+bold}generate{/$} ({[white]}{[blue]}{[green]}{[yellow]}{[red]} {[green]})",
            empty_s,
            string_s,
            yaml_s,
            toml_s,
            json_s,
            dest.display()
        );

        // Normalize dest.
        let dest_w = self.join_dest(dest);

        match typ {
            GeneratedFileTyp::Empty(_) => Action::Write(WriteAction {
                dest: dest_w,
                contents: "".to_string(),
            }),
            GeneratedFileTyp::String(s) => Action::Write(WriteAction {
                dest: dest_w,
                contents: s.contents.clone(),
            }),
            // FIXME error context
            GeneratedFileTyp::Yaml(y) => Action::Yaml(YamlAction {
                dest: dest_w,
                values: y.values.clone(),
                header: y.header.clone(),
            }),
            GeneratedFileTyp::Toml(t) => Action::Toml(TomlAction {
                dest: dest_w,
                values: t.values.clone(),
                header: t.header.clone(),
            }),
            GeneratedFileTyp::Json(j) => Action::Json(JsonAction {
                dest: dest_w,
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
            start,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit,
        } = cmd;

        // Use sh as default shell.
        let shell = shell.as_ref().map(String::as_str).unwrap_or("sh");
        idx_debug!(
            self.i,
            self.n,
            "{$blue+bold}hook{/$} ({$white}shell{/$} '{[dimmed]})",
            command
        );

        // Normalize start path.
        let start_w = start
            .as_ref()
            .map(|start| self.join_package(start))
            .unwrap_or(self.path.clone());

        let command = command.clone();
        let shell = shell.to_string();
        let stdout = *stdout.as_ref().unwrap_or(&true);
        let stderr = *stderr.as_ref().unwrap_or(&true);
        let clean_env = *clean_env.as_ref().unwrap_or(&false);
        let env = env.clone().unwrap_or_else(|| EnvMap::new());
        let nonzero_exit = nonzero_exit.clone().unwrap_or(NonZeroExitBehavior::Ignore);

        Action::Command(CommandAction {
            command,
            start: start_w,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit,
        })
    }

    #[inline]
    fn convert_hook_fun(&self, fun: &FunHook) -> Action<'p> {
        let FunHook {
            name,
            start,
            error_exit,
        } = fun;

        idx_debug!(
            self.i,
            self.n,
            "{$blue+bold}hook{/$} ({$white}fn{/$} '{$dimmed+italic}<function>{/$})"
        );

        // Load function from Lua registry.
        let function: Function = self.lua.named_registry_value(name).unwrap();
        let start = start
            .as_ref()
            .map(|start| self.join_package(start))
            .unwrap_or_else(|| self.path.clone());

        Action::Function(FunctionAction {
            function,
            start,
            error_exit: error_exit.clone().unwrap_or(NonZeroExitBehavior::Ignore),
        })
    }

    #[inline]
    fn join_package<P>(&self, path: P) -> PathWrapper
    where
        P: AsRef<Path>,
    {
        self.normalize_path(path, &self.path.abs())
    }

    #[inline]
    fn join_dest<P>(&self, path: P) -> PathWrapper
    where
        P: AsRef<Path>,
    {
        self.normalize_path(path, &self.dest.abs())
    }

    #[inline]
    fn normalize_path<P, S>(&self, path: P, start: S) -> PathWrapper
    where
        P: AsRef<Path>,
        S: AsRef<Path>,
    {
        PathWrapper::from_with_start(path.as_ref().to_path_buf(), start)
    }

    #[inline]
    fn log_processing(&self, step: &str) {}
}
