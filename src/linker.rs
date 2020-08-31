use crate::config::{Config, FileProcess, Hook, LinkType, Package, TemplateProcess};
use crate::map::Map;
use crate::symlink;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use log::{debug, info, trace};

#[derive(Debug)]
pub struct Linker {
    dest: PathBuf,
    path: PathBuf,
    package: Package,
    quiet: bool,
    verbosity: usize,

    dependency_linkers: Vec<Self>,
    extension_linkers: Vec<Self>,
}

impl Linker {
    pub fn new<P: AsRef<Path>>(
        package: Package,
        path: P,
        dest: P,
        quiet: bool,
        verbosity: usize,
    ) -> Result<Self> {
        let mut new = Self {
            package,
            path: path.as_ref().into(),
            dest: dest.as_ref().into(),
            quiet,
            verbosity,
            dependency_linkers: vec![],
            extension_linkers: vec![],
        };

        new.dependency_linkers = new.parse_dependency_linkers()?;
        new.extension_linkers = new.parse_extension_linkers()?;

        Ok(new)
    }

    pub fn from_path<P: AsRef<Path>>(
        dir_path: P,
        dest: P,
        quiet: bool,
        verbosity: usize,
    ) -> Result<Self> {
        let package = Package::from_dhall_file(dir_path.as_ref().join("package.dhall"))
            .with_context(|| format!("Failed to parse package configuration"))?;
        Self::new(package, dir_path, dest, quiet, verbosity)
    }

    pub fn link(&self) -> Result<()> {
        info!("Linking {}...", self.package_cfg().name);

        // Work relative to the package root.
        let cwd = env::current_dir().with_context(|| "Failed to determine current directory")?;
        env::set_current_dir(self.path.clone())
            .with_context(|| "Failed to change working directory")?;

        debug!("Linking dependencies...");
        self.link_dependencies()?;

        debug!("Running before-link hooks...");
        self.exec_before_link()?;

        debug!("Linking files...");
        self.link_files()?;

        debug!("Running after-link hooks...");
        self.exec_after_link()?;

        debug!("Linking extensions...");
        self.link_extensions()?;

        env::set_current_dir(cwd).with_context(|| "Failed to revert working directory")?;

        Ok(())
    }

    pub fn link_files(&self) -> Result<()> {
        let normal_link_paths = self.package.normal_link_paths()?;
        for link_path in normal_link_paths {
            self.normal_link_file(&link_path)?;
        }

        for file_process in &self.package_cfg().files {
            self.link_file_process(&file_process)?;
        }

        for template_process in &self.package_cfg().template_files {
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
        let dest = self.dest.join(relative_to_tree);

        self.link_file(absolute, dest, &self.package.config.default_link_type)
    }

    fn link_file_process(&self, file_process: &FileProcess) -> Result<()> {
        let src = PathBuf::from(&file_process.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &file_process.dest;
        let absolute_dest = self.dest.join(dest);

        self.link_file(absolute_src, absolute_dest, &file_process.link_type)
    }

    fn link_template_process(&self, template_process: &TemplateProcess) -> Result<()> {
        let src = PathBuf::from(&template_process.src);
        let absolute_src = fs::canonicalize(src.clone())
            .with_context(|| format!("Failed to determine absolute path of {}", src.display()))?;

        let dest = &template_process.dest;
        let absolute_dest = self.dest.join(dest);

        let src_str = fs::read_to_string(absolute_src.clone())
            .with_context(|| format!("Failed to read source file {}", absolute_src.display()))?;
        let rendered_str = Templater::render(&src_str, self.package_variables().map.clone())
            .with_context(|| {
                format!("Failed to render template file: {}", absolute_src.display())
            })?;

        self.prepare_link_location(&absolute_dest)?;
        fs::write(&absolute_dest, rendered_str)
            .with_context(|| format!("Failed to write file {}", absolute_dest.display()))
    }

    /// Symlink or copy a file. `src` and `dest` can be absolute paths, or relative to the package root.
    fn link_file<P: AsRef<Path> + Clone>(
        &self,
        src: P,
        dest: P,
        link_type: &LinkType,
    ) -> Result<()> {
        trace!(
            "Linking {} -> {}",
            src.as_ref().display(),
            dest.as_ref().display()
        );

        self.prepare_link_location(&dest)?;

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
                fs::copy(&src, &dest).with_context(|| {
                    format!(
                        "Failed to copy from {} to {}",
                        src.as_ref().display(),
                        dest.as_ref().display()
                    )
                })?;
            }
        };

        Ok(())
    }

    fn prepare_link_location<P: AsRef<Path>>(&self, dest: P) -> Result<()> {
        let dest = dest.as_ref();
        if dest.exists() {
            // If dest exists, check if it is a file or directory.
            if dest.is_file() {
                if self.package.config.replace_files {
                    fs::remove_file(dest.clone())
                        .with_context(|| format!("Failed to remove file at {}", dest.display()))
                } else {
                    Err(anyhow!("{} is an existing file", dest.display()))
                }
            } else if dest.is_dir() {
                if self.package.config.replace_directories {
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
        self.exec_hooks(&self.package_cfg().before_link)?;
        Ok(())
    }

    pub fn exec_after_link(&self) -> Result<()> {
        self.exec_hooks(&self.package_cfg().after_link)?;
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
        if !self.quiet {
            cmd.stderr(Stdio::inherit());
            if self.verbosity > 2 {
                cmd.stdout(Stdio::inherit());
            }
        }

        let child = cmd.spawn().with_context(|| "Failed to spawn command")?;
        child
            .wait_with_output()
            .with_context(|| "Failed to with on process")?;

        Ok(())
    }

    pub fn link_dependencies(&self) -> Result<()> {
        for linker in &self.dependency_linkers {
            linker.link()?;
        }
        Ok(())
    }

    pub fn link_extensions(&self) -> Result<()> {
        for linker in &self.extension_linkers {
            linker.link()?;
        }
        Ok(())
    }

    /// Returns Linkers for the dependencies.
    fn parse_dependency_linkers(&self) -> Result<Vec<Self>> {
        let dependencies = &self.package_cfg().dependencies;
        self.linkers_list(dependencies)
    }

    /// Returns Linkers for the exntension packages.
    fn parse_extension_linkers(&self) -> Result<Vec<Self>> {
        let extensions = &self.package_cfg().extensions;
        self.linkers_list(extensions)
    }

    /// Returns Linkers for the dependencies/extensions of the package.
    fn linkers_list<P: AsRef<Path>>(&self, paths: &Vec<P>) -> Result<Vec<Self>> {
        paths
            .iter()
            .map(|dep| -> Result<_> {
                Self::from_path(
                    dep.as_ref().into(),
                    self.dest.clone(),
                    self.quiet,
                    self.verbosity,
                )
            })
            .collect::<Result<Vec<Self>>>()
    }

    fn package_cfg<'a>(&'a self) -> &'a Config {
        &self.package.config
    }

    fn package_variables<'a>(&'a self) -> &'a Map {
        &self.package.variables
    }
}

#[derive(Debug)]
struct Templater;

impl Templater {
    fn render<T: Into<gtmpl::Value>>(template: &str, context: T) -> Result<String> {
        let rendered = gtmpl::template(template, context)
            .map_err(|err| anyhow!("Failed to render template: {}", err))?;
        Ok(rendered)
    }
}
