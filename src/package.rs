use crate::config;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use log::{debug, info};

#[derive(Debug)]
struct Stew;

#[derive(Debug)]
pub struct Package {
    pub path: PathBuf,
    pub cfg: config::Package,
}

impl Package {
    pub fn new(path: PathBuf, cfg: config::Package) -> Result<Self> {
        Ok(Self {
            path: fs::canonicalize(path)?,
            cfg,
        })
    }

    pub fn from_path(path: PathBuf) -> Result<Self> {
        let cfg_path = path.join("package.dhall");
        let cfg = config::Package::from_dhall_file(&cfg_path)?;
        Self::new(path, cfg)
    }

    pub fn tree_path(&self) -> PathBuf {
        self.path.join("tree")
    }

    pub fn tree_file_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.tree_path().join(path)
    }

    pub fn file_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.path.join(path)
    }
}

#[derive(Debug)]
pub struct Linker {
    package: Package,
    quiet: bool,
    verbosity: usize,
}

impl Linker {
    pub fn new(package: Package, quiet: bool, verbosity: usize) -> Self {
        Self {
            package,
            quiet,
            verbosity,
        }
    }

    pub fn link(&self) -> Result<()> {
        info!("Linking {}...", self.cfg().name);

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

        Ok(())
    }

    pub fn link_files(&self) -> Result<()> {
        Ok(())
    }

    pub fn exec_before_link(&self) -> Result<()> {
        self.exec_hooks(&self.cfg().before_link)?;
        Ok(())
    }

    pub fn exec_after_link(&self) -> Result<()> {
        self.exec_hooks(&self.cfg().after_link)?;
        Ok(())
    }

    fn exec_hooks(&self, hooks: &Vec<config::Hook>) -> Result<()> {
        for hook in hooks {
            self.exec_hook(&hook)?;
        }
        Ok(())
    }

    fn exec_hook(&self, hook: &config::Hook) -> Result<()> {
        let parts = match shlex::split(&hook.string) {
            Some(v) => v,
            None => return Err(anyhow!("Failed to run hook {}", hook.name)),
        };

        let bin = match parts.get(0) {
            Some(p) => self.package.file_path(&p),
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

    fn dependency_linkers(&self) -> Result<Vec<Self>> {
        let dependencies = &self.cfg().dependencies;
        self.linkers_list(dependencies)
    }

    pub fn link_extensions(&self) -> Result<()> {
        let linkers = self.extension_linkers()?;
        for linker in linkers {
            linker.link()?;
        }
        Ok(())
    }

    fn extension_linkers(&self) -> Result<Vec<Self>> {
        let extensions = &self.cfg().extensions;
        self.linkers_list(extensions)
    }

    fn linkers_list<P: AsRef<Path>>(&self, paths: &Vec<P>) -> Result<Vec<Self>> {
        paths
            .iter()
            .map(|dep| -> Result<_> {
                let path = self.package.file_path(dep);
                let dep_package = Package::from_path(path)?;
                Ok(Linker::new(dep_package, self.quiet, self.verbosity))
            })
            .collect::<Result<Vec<Self>>>()
    }

    fn cfg<'a>(&'a self) -> &'a config::Package {
        &self.package.cfg
    }
}
