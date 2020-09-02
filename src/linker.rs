use crate::config::{FileProcess, Hook, LinkType, Package, TemplateProcess};
use crate::dependency::PackageGraph;
use crate::symlink;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use log::{debug, info, trace};

#[derive(Debug)]
struct Templater;

impl Templater {
    fn render<T: Into<gtmpl::Value>>(template: &str, context: T) -> Result<String> {
        let rendered = gtmpl::template(template, context)
            .map_err(|err| anyhow!("Failed to render template: {}", err))?;
        Ok(rendered)
    }
}

#[derive(Debug)]
pub struct Linker {
    dest: PathBuf,
    quiet: bool,
    verbosity: usize,
}

impl Linker {
    pub fn new<P: AsRef<Path>>(dest: P, quiet: bool, verbosity: usize) -> Self {
        Self {
            dest: dest.as_ref().into(),
            quiet,
            verbosity,
        }
    }

    pub fn link(&self, path: PathBuf, package: Package) -> Result<()> {
        info!("Resolving package dependency graph...");
        let graph = PackageGraph::from_package(path, package)?;

        info!("Sorting packages...");
        let order = graph.topological_order()?;

        for (path, package) in order {
            info!("Linking {}", package.config.name);
            let state = LinkerState::new(self, path, package);
            state.link()?;
        }

        Ok(())
    }
}

struct LinkerState<'a> {
    linker: &'a Linker,
    path: &'a PathBuf,
    package: &'a Package,
}

impl<'a> LinkerState<'a> {
    fn new(linker: &'a Linker, path: &'a PathBuf, package: &'a Package) -> Self {
        Self {
            linker,
            path,
            package,
        }
    }
}

impl<'a> LinkerState<'a> {
    fn link(&self) -> Result<()> {
        // Work relative to the package root.
        let cwd = env::current_dir().with_context(|| "Failed to determine current directory")?;
        env::set_current_dir(self.path.clone())
            .with_context(|| "Failed to change working directory")?;

        debug!("Running before-link hooks...");
        self.exec_before_link()?;

        debug!("Linking files...");
        self.link_files()?;

        debug!("Running after-link hooks...");
        self.exec_after_link()?;

        env::set_current_dir(cwd).with_context(|| "Failed to revert working directory")?;

        Ok(())
    }

    fn link_files(&self) -> Result<()> {
        let normal_link_paths = self.package.normal_link_paths()?;
        for link_path in normal_link_paths {
            self.normal_link_file(&link_path)?;
        }

        for file_process in &self.package.config.files {
            self.link_file_process(&file_process)?;
        }

        for template_process in &self.package.config.template_files {
            self.link_template_process(&template_process)?;
        }

        Ok(())
    }

    /// Link a file relative to the package root to its proper location.
    fn normal_link_file<P: AsRef<Path> + Clone>(&self, path: P) -> Result<()> {
        let absolute = fs::canonicalize(path.clone()).with_context(|| {
            format!(
                "Failed to determine absolute path of {}",
                path.as_ref().display()
            )
        })?;

        let relative_to_tree = path.as_ref().strip_prefix(self.package.tree_path())?;
        let dest = self.linker.dest.join(relative_to_tree);

        self.link_file(
            absolute,
            dest,
            &self.package.config.default_link_type,
            self.package.config.replace_files,
            self.package.config.replace_directories,
        )
    }

    fn link_file_process(&self, file_process: &FileProcess) -> Result<()> {
        let src = PathBuf::from(&file_process.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &file_process.dest;
        let absolute_dest = self.linker.dest.join(dest);

        let replace_files = match file_process.replace_files {
            Some(b) => b,
            None => self.package.config.replace_files,
        };

        let replace_directories = match file_process.replace_directories {
            Some(b) => b,
            None => self.package.config.replace_directories,
        };

        self.link_file(
            absolute_src,
            absolute_dest,
            &file_process.link_type,
            replace_files,
            replace_directories,
        )
    }

    fn link_template_process(&self, template_process: &TemplateProcess) -> Result<()> {
        let src = PathBuf::from(&template_process.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &template_process.dest;
        let absolute_dest = self.linker.dest.join(dest);

        let src_str = fs::read_to_string(absolute_src.clone())
            .with_context(|| format!("Failed to read source file {}", absolute_src.display()))?;
        let rendered_str = Templater::render(&src_str, self.package.variables.map.clone())
            .with_context(|| {
                format!("Failed to render template file: {}", absolute_src.display())
            })?;

        let replace_files = match template_process.replace_files {
            Some(b) => b,
            None => self.package.config.replace_files,
        };

        let replace_directories = match template_process.replace_directories {
            Some(b) => b,
            None => self.package.config.replace_directories,
        };

        self.prepare_link_location(&absolute_dest, replace_files, replace_directories)?;
        fs::write(&absolute_dest, rendered_str)
            .with_context(|| format!("Failed to write file {}", absolute_dest.display()))
    }

    /// Symlink or copy a file. `src` and `dest` can be absolute paths, or relative to the package root.
    fn link_file<P: AsRef<Path> + Clone>(
        &self,
        src: P,
        dest: P,
        link_type: &LinkType,
        replace_files: bool,
        replace_directories: bool,
    ) -> Result<()> {
        trace!(
            "Linking {} -> {}",
            src.as_ref().display(),
            dest.as_ref().display()
        );

        self.prepare_link_location(&dest, replace_files, replace_directories)?;

        match *link_type {
            LinkType::Link => {
                symlink::symlink(&src, &dest).with_context(|| {
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
                    fs::remove_file(dest.clone())
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
                .ok_or(anyhow!(
                    "Failed to determine parent directory of {}",
                    dest.display()
                ))?
                .into();

            if dest_parent.is_dir() {
                return Ok(());
            } else {
                // Otherwise, try to create directories recursively and write to a new
                // file.
                return fs::create_dir_all(dest_parent.clone()).with_context(|| {
                    format!("Failed to create directories for {}", dest_parent.display(),)
                });
            }
        }
    }

    pub fn exec_before_link(&self) -> Result<()> {
        self.exec_hooks(&self.package.config.before_link)?;
        Ok(())
    }

    pub fn exec_after_link(&self) -> Result<()> {
        self.exec_hooks(&self.package.config.after_link)?;
        Ok(())
    }

    /// Executes a list of hook commands.
    fn exec_hooks(&self, hooks: &Vec<Hook>) -> Result<()> {
        for hook in hooks {
            debug!("Running hook {}...", hook.name);
            self.exec_hook(&hook)?;
        }
        Ok(())
    }

    /// Executes a hook command.
    fn exec_hook(&self, hook: &Hook) -> Result<()> {
        let parts = match shlex::split(&hook.string) {
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
