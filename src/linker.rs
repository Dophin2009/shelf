use crate::config::{Config, Hook, LinkType, Package};
use crate::map::Map;
use crate::symlink;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use log::{debug, info};

#[derive(Debug)]
pub struct Linker {
    dest: PathBuf,
    path: PathBuf,
    package: Package,
    quiet: bool,
    verbosity: usize,
}

impl Linker {
    pub fn new(
        package: Package,
        path: PathBuf,
        dest: PathBuf,
        quiet: bool,
        verbosity: usize,
    ) -> Self {
        Self {
            package,
            path,
            dest,
            quiet,
            verbosity,
        }
    }

    pub fn link(&self) -> Result<()> {
        info!("Linking {}...", self.package_cfg().name);

        // Work relative to the package root.
        let cwd = env::current_dir()?;
        env::set_current_dir(self.path.clone())?;

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

        env::set_current_dir(cwd)?;

        Ok(())
    }

    pub fn link_files(&self) -> Result<()> {
        let normal_link_paths = self.package.normal_link_paths()?;
        for link_path in normal_link_paths {
            self.link_file(&link_path, &self.package.config.default_link_type)?;
        }

        Ok(())
    }

    /// Link a file relative to the package root to its proper location.
    fn link_file<P: AsRef<Path> + Clone>(&self, path: P, link_type: &LinkType) -> Result<()> {
        let absolute = fs::canonicalize(path.clone())?;

        let relative_to_tree = path.as_ref().strip_prefix(self.package.tree_path())?;
        let dest = self.dest.join(relative_to_tree);

        println!("{} -> {}", absolute.display(), dest.display());

        self.prepare_link_location(&dest)?;

        match *link_type {
            LinkType::Link => {
                symlink::symlink(&absolute, &dest)
                    .map_err(|err| anyhow!("Failed to create symlink: {}", err))?;
            }
            LinkType::Copy => {
                fs::copy(&absolute, &dest)
                    .map_err(|err| anyhow!("Failed to copy file: {}", err))?;
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
                        .map_err(|err| anyhow!("Failed to remove file: {}", err))
                } else {
                    Err(anyhow!("{} is an existing file", dest.display()))
                }
            } else if dest.is_dir() {
                if self.package.config.replace_directories {
                    fs::remove_dir_all(dest)
                        .map_err(|err| anyhow!("Failed to remove directory: {}", err))
                } else {
                    Err(anyhow!("{} is an existing directory", dest.display()))
                }
            } else {
                // Otherwise, return error.
                Err(anyhow!("Could not stat file at {}", dest.display()))
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
                return fs::create_dir_all(dest_parent.clone()).map_err(|err| {
                    anyhow!(
                        "Failed to create directories for {}: {}",
                        dest_parent.display(),
                        err
                    )
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

        let child = cmd.spawn()?;
        child.wait_with_output()?;

        Ok(())
    }

    pub fn link_dependencies(&self) -> Result<()> {
        let linkers = self.dependency_linkers()?;
        for linker in linkers {
            linker.link()?;
        }
        Ok(())
    }

    /// Returns Linkers for the dependencies.
    fn dependency_linkers(&self) -> Result<Vec<Self>> {
        let dependencies = &self.package_cfg().dependencies;
        self.linkers_list(dependencies)
    }

    pub fn link_extensions(&self) -> Result<()> {
        let linkers = self.extension_linkers()?;
        for linker in linkers {
            linker.link()?;
        }
        Ok(())
    }

    /// Returns Linkers for the exntension packages.
    fn extension_linkers(&self) -> Result<Vec<Self>> {
        let extensions = &self.package_cfg().extensions;
        self.linkers_list(extensions)
    }

    /// Returns Linkers for the dependencies of the package.
    fn linkers_list<P: AsRef<Path>>(&self, paths: &Vec<P>) -> Result<Vec<Self>> {
        paths
            .iter()
            .map(|dep| -> Result<_> {
                let dep_package = Package::from_dir(dep.as_ref().into())?;
                Ok(Linker::new(
                    dep_package,
                    dep.as_ref().into(),
                    self.dest.clone(),
                    self.quiet,
                    self.verbosity,
                ))
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
