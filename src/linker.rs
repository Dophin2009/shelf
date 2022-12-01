use crate::loader::{PackageGraph, PackageState};
use crate::package::{File, Hook, HookBody, LinkType, Package, Template, TemplateType, Tree};
use crate::template::{gotmpl, hbs, tera};

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use log::{debug, info, trace};
use mlua::{Function as LuaFunction, Lua};

#[derive(Debug)]
pub struct Linker {
    dest: PathBuf,
    quiet: bool,
    verbosity: u8,
}

impl Linker {
    pub fn new<P: AsRef<Path>>(dest: P, quiet: bool, verbosity: u8) -> Self {
        Self {
            dest: dest.as_ref().into(),
            quiet,
            verbosity,
        }
    }

    // TODO: Verify paths exist and are valid before actually linking.
    // TODO: Make logging less hacky.
    pub fn link(&self, graph: &PackageGraph) -> Result<()> {
        self.link_internal(graph, false)
    }

    pub fn link_noop(&self, graph: &PackageGraph) -> Result<()> {
        self.link_internal(graph, true)
    }

    fn link_internal(&self, graph: &PackageGraph, noop: bool) -> Result<()> {
        info!("Sorting packages...");
        let order = graph.topological_order()?;
        for package in order {
            info!("Linking {}", package.data.name);
            let state = LinkerState::new(self, package, noop);
            state.link()?;
        }

        Ok(())
    }
}

struct LinkerState<'a> {
    linker: &'a Linker,
    path: &'a PathBuf,
    package: &'a Package,
    lua: &'a Lua,
    noop: bool,
}

impl<'a> LinkerState<'a> {
    fn new(linker: &'a Linker, package: &'a PackageState, noop: bool) -> Self {
        Self {
            linker,
            path: &package.path,
            package: &package.data,
            lua: &package.lua,
            noop,
        }
    }
}

impl<'a> LinkerState<'a> {
    fn link(&self) -> Result<()> {
        // Work relative to the package root.
        let cwd = env::current_dir().with_context(|| "Failed to determine current directory")?;
        env::set_current_dir(self.path.clone())
            .with_context(|| "Failed to change working directory")?;

        debug!("-- Running before-link hooks...");
        self.exec_pre()?;

        self.link_files()?;

        debug!("-- Running after-link hooks...");
        self.exec_post()?;

        env::set_current_dir(cwd).with_context(|| "Failed to revert working directory")?;

        Ok(())
    }

    fn link_files(&self) -> Result<()> {
        let files = &self.package.files;

        // Link trees.
        debug!("-- Linking trees...");
        for tree in &files.trees {
            self.link_tree(&tree)?;
        }

        // Link extra files.
        debug!("-- Linking extra files...");
        for extra in &files.extra {
            self.link_extra(extra)?;
        }

        // Link template files.
        debug!("-- Linking templates...");
        for template in &files.templates {
            self.link_template(&template)?;
        }

        Ok(())
    }

    /// Link a file relative to the package root to its proper location.
    fn link_tree(&self, tree: &Tree) -> Result<()> {
        // Get normal link paths with ignore patterns applied.
        let normal_link_paths = self.normal_link_paths(tree)?;
        for path in normal_link_paths {
            let absolute = fs::canonicalize(path.clone()).with_context(|| {
                format!("Failed to determine absolute path of {}", path.display())
            })?;

            let tree_absolute = fs::canonicalize(&tree.path)?;
            let rel_to_tree = absolute.strip_prefix(&tree_absolute)?;
            let dest = self.linker.dest.join(rel_to_tree);

            self.link_file(
                absolute,
                dest,
                &tree.link_type,
                &tree.replace_files,
                &tree.replace_dirs,
            )?;
        }
        Ok(())
    }

    fn link_extra(&self, file: &File) -> Result<()> {
        let src = PathBuf::from(&file.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &file.dest;
        let absolute_dest = self.linker.dest.join(dest);

        self.link_file(
            absolute_src,
            absolute_dest,
            &file.link_type,
            &file.replace_files,
            &file.replace_dirs,
        )
    }

    fn link_template(&self, template: &Template) -> Result<()> {
        let src = PathBuf::from(&template.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &template.dest;
        let absolute_dest = self.linker.dest.join(dest);

        let src_str = fs::read_to_string(absolute_src.clone())
            .with_context(|| format!("Failed to read source file {}", absolute_src.display()))?;
        let rendered_result = match template.ty {
            TemplateType::Gotmpl => gotmpl::render(&src_str, self.package.variables.map.clone()),
            TemplateType::Tera => tera::render(&src_str, &self.package.variables.map),
            TemplateType::Handlebars { ref partials } => {
                hbs::render(&src_str, &self.package.variables.map, partials)
            }
        };
        let rendered_str = rendered_result.with_context(|| {
            format!("Failed to render template file: {}", absolute_src.display())
        })?;

        let replace_files = template
            .replace_files
            .unwrap_or_else(|| self.package.files.replace_files);
        let replace_dirs = template
            .replace_dirs
            .unwrap_or_else(|| self.package.files.replace_dirs);

        trace!(
            "-- -- Templating {} -> {}",
            absolute_src.display(),
            absolute_dest.display()
        );
        if !self.noop {
            self.prepare_link_location(&absolute_dest, replace_files, replace_dirs)?;
            fs::write(&absolute_dest, rendered_str)
                .with_context(|| format!("Failed to write file {}", absolute_dest.display()))?;
        }

        Ok(())
    }

    /// Symlink or copy a file. `src` and `dest` can be absolute paths, or relative to the package root.
    fn link_file<P: AsRef<Path> + Clone>(
        &self,
        src: P,
        dest: P,
        link_type: &Option<LinkType>,
        replace_files: &Option<bool>,
        replace_directories: &Option<bool>,
    ) -> Result<()> {
        trace!(
            "-- -- Linking {} -> {}",
            src.as_ref().display(),
            dest.as_ref().display()
        );

        let link_type = match link_type {
            Some(t) => t,
            None => &self.package.files.link_type,
        };
        let replace_files = replace_files.unwrap_or_else(|| self.package.files.replace_files);
        let replace_directories =
            replace_directories.unwrap_or_else(|| self.package.files.replace_dirs);

        if !self.noop {
            self.prepare_link_location(&dest, replace_files, replace_directories)?;

            match link_type {
                LinkType::Link => {
                    symlink(&src, &dest).with_context(|| {
                        format!(
                            "Failed to create symlink between {} and {}",
                            src.as_ref().display(),
                            dest.as_ref().display()
                        )
                    })?;
                }
                LinkType::Copy => {
                    if src.as_ref().is_file() {
                        fs::copy(&src, &dest).with_context(|| {
                            format!(
                                "Failed to copy from {} to {}",
                                src.as_ref().display(),
                                dest.as_ref().display()
                            )
                        })?;
                    } else if src.as_ref().is_dir() {
                        self.copy_dir(&src, &dest).with_context(|| {
                            format!(
                                "Failed to copy dir from {} to {}",
                                src.as_ref().display(),
                                dest.as_ref().display()
                            )
                        })?;
                    } else {
                        return Err(anyhow!("Cannot copy from path: {}", src.as_ref().display()));
                    }
                }
            };
        }

        Ok(())
    }

    pub fn copy_dir<U: AsRef<Path>, V: AsRef<Path>>(&self, from: U, to: V) -> Result<()> {
        let mut stack = Vec::new();
        stack.push(PathBuf::from(from.as_ref()));

        let output_root = PathBuf::from(to.as_ref());
        let input_root = PathBuf::from(from.as_ref()).components().count();

        while let Some(working_path) = stack.pop() {
            // Generate a relative path
            let src: PathBuf = working_path.components().skip(input_root).collect();

            // Create a destination if missing
            let dest = if src.components().count() == 0 {
                output_root.clone()
            } else {
                output_root.join(&src)
            };

            if fs::metadata(&dest).is_err() {
                fs::create_dir_all(&dest)?;
            }

            for entry in fs::read_dir(working_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    match path.file_name() {
                        Some(filename) => {
                            let dest_path = dest.join(filename);
                            fs::copy(&path, &dest_path)?;
                        }
                        None => return Err(anyhow!("Failed to copy: {}", path.display())),
                    }
                }
            }
        }

        Ok(())
    }

    fn prepare_link_location<P: AsRef<Path>>(
        &self,
        dest: P,
        replace_files: bool,
        replace_directories: bool,
    ) -> Result<()> {
        let dest = dest.as_ref();
        if dest.exists() {
            // If dest exists, check if it is a file or directory.
            if dest.is_file() {
                if replace_files {
                    fs::remove_file(dest)
                        .with_context(|| format!("Failed to remove file at {}", dest.display()))
                } else {
                    Err(anyhow!("{} is an existing file", dest.display()))
                }
            } else if dest.is_dir() {
                if replace_directories {
                    fs::remove_dir_all(dest)
                        .map_err(|err| anyhow!("Failed to remove directory at {}", err))
                } else {
                    Err(anyhow!("{} is an existing directory", dest.display()))
                }
            } else {
                // Otherwise, return error.
                Err(anyhow!("Failed to stat file at {}", dest.display()))
            }
        } else {
            // If dest doesn't exist, check its parent.
            let dest_parent: PathBuf = dest
                .parent()
                .ok_or_else(|| {
                    anyhow!("Failed to determine parent directory of {}", dest.display())
                })?
                .into();

            if dest_parent.is_dir() {
                Ok(())
            } else {
                // Otherwise, try to create directories recursively and write to a new
                // file.
                fs::create_dir_all(dest_parent.clone()).with_context(|| {
                    format!("Failed to create directories for {}", dest_parent.display(),)
                })
            }
        }
    }

    /// Returns the set of paths (relative to the package root), with ignore patterns applied.
    pub fn normal_link_paths(&self, tree: &Tree) -> Result<HashSet<PathBuf>> {
        // Glob all files starting at tree.
        let mut paths: HashSet<_> = self.glob_relative(&tree.file_path_str("**/*"))?;

        // Glob ignore patterns.
        let ignored = self.ignored_paths(tree)?;

        // Remove ignored paths.
        for path in HashSet::intersection(&paths.clone(), &ignored) {
            paths.remove(path);
        }

        Ok(paths)
    }

    /// Returns the set of paths (relative to the package root) that should be ignored in normal
    /// linking.
    pub fn ignored_paths(&self, tree: &Tree) -> Result<HashSet<PathBuf>> {
        // Process global ignore.
        let paths: Vec<HashSet<PathBuf>> = tree
            .ignore
            .iter()
            .map(|p| self.glob_relative(&tree.file_path_str(p)))
            .collect::<Result<_>>()?;

        Ok(paths
            .into_iter()
            .flat_map(|set| set)
            .collect::<HashSet<_>>())
    }

    /// Glob for files with a pattern relative to the package root.
    fn glob_relative(&self, pattern: &str) -> Result<HashSet<PathBuf>> {
        let paths = glob::glob(pattern)
            .with_context(|| format!("Failed to glob invalid pattern {}", pattern))?;

        // Collect and check the glob results.
        let mut set: HashSet<PathBuf> = paths
            .map(|glob_result| {
                glob_result
                    .with_context(|| format!("Failed to stat path in globbing pattern {}", pattern))
            })
            .collect::<Result<_>>()?;

        // Filter to only include files.
        set = set.into_iter().filter(|p| p.is_file()).collect();

        Ok(set)
    }

    pub fn exec_pre(&self) -> Result<()> {
        self.exec_hooks(&self.package.hooks.pre)?;
        Ok(())
    }

    pub fn exec_post(&self) -> Result<()> {
        self.exec_hooks(&self.package.hooks.post)?;
        Ok(())
    }

    /// Executes a list of hook commands.
    fn exec_hooks(&self, hooks: &[Hook]) -> Result<()> {
        for hook in hooks {
            debug!("-- Running hook {}...", hook.name);
            if !self.noop {
                self.exec_hook(&hook)?;
            }
        }
        Ok(())
    }

    /// Executes a hook command.
    fn exec_hook(&self, hook: &Hook) -> Result<()> {
        match &hook.body {
            HookBody::Executable { command } => {
                let parts = match shlex::split(&command) {
                    Some(v) => v,
                    None => {
                        return Err(anyhow!(
                            "Failed to run hook {}: invalid invocation",
                            hook.name
                        ))
                    }
                };

                let bin = match parts.get(0) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let args = &parts[1..];

                let mut cmd = Command::new(bin);
                cmd.args(args);

                self.exec_command(cmd)
                    .map_err(|err| anyhow!("Failed to run hook {}: {}", hook.name, err))
            }
            HookBody::LuaFunction { name } => {
                let func: LuaFunction = self.lua.named_registry_value(&name)?;
                func.call::<(), ()>(())?;

                Ok(())
            }
        }
    }

    fn exec_command(&self, mut cmd: Command) -> Result<()> {
        if !self.linker.quiet {
            cmd.stderr(Stdio::inherit());
            if self.linker.verbosity >= 4 {
                cmd.stdout(Stdio::inherit());
            }
        }

        let child = cmd.spawn().with_context(|| "Failed to spawn command")?;
        child
            .wait_with_output()
            .with_context(|| "Failed to with on process")?;

        Ok(())
    }
}

#[cfg(windows)]
pub fn symlink<P: AsRef<Path>>(src: P, dest: P) -> io::Result<()> {
    use std::os::windows;
    windows::fs::symlink_file(src, dest)
}

#[cfg(unix)]
pub fn symlink<P: AsRef<Path>>(src: P, dest: P) -> io::Result<()> {
    use std::os::unix;
    unix::fs::symlink(src, dest)
}
