use crate::map::Map;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Package {
    #[serde(default)]
    pub variables: Map,
    #[serde(rename = "package")]
    pub config: Config,
}

impl Package {
    pub fn from_dhall_file<P: AsRef<Path>>(path: P) -> serde_dhall::Result<Self> {
        serde_dhall::from_file(path).parse()
    }

    /// Returns the set of paths (relative to the package root) that should be ignored in normal
    /// linking.
    pub fn normal_link_paths(&self) -> Result<HashSet<PathBuf>> {
        // Glob all files starting at tree.
        let mut paths: HashSet<_> = self.glob_relative(&self.tree_path_str("**/*"))?;

        // Glob ignore patterns.
        let ignored = self.ignored_paths()?;

        // Remove ignored paths.
        for path in HashSet::intersection(&paths.clone(), &ignored) {
            paths.remove(path);
        }

        for file_process in &self.config.files {
            let path: PathBuf = file_process.src.clone().into();
            paths.remove(&path);

            let descendants = self.glob_relative(&path.join("**/*").to_string_lossy())?;
            for d in descendants {
                paths.remove(&d);
            }
        }

        for template_process in &self.config.template_files {
            let path: PathBuf = template_process.src.clone().into();
            paths.remove(&path);
        }

        Ok(paths)
    }

    /// Returns the set of paths (relative to the package root) that should be ignored in normal
    /// linking.
    pub fn ignored_paths(&self) -> Result<HashSet<PathBuf>> {
        let ignore_patterns = &self.config.ignore_patterns;
        let paths_iter = ignore_patterns
            .iter()
            .map(|p| self.glob_relative(&self.tree_path_str(p)));

        let mut paths = HashSet::new();
        for p_result in paths_iter {
            let p = p_result?;
            paths.extend(p);
        }

        Ok(paths)
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

    /// Get the path of some path (relative to the tree root) relative to the project root.
    pub fn tree_file_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.tree_path().join(path)
    }

    /// Get the path of the tree root relative to the project root.
    pub fn tree_path(&self) -> PathBuf {
        self.tree_prefix().into()
    }

    fn tree_path_str(&self, s: &str) -> String {
        let p = if s.starts_with("/") {
            s.chars().skip(1).collect()
        } else {
            String::from(s)
        };

        format!("{}/{}", self.tree_prefix(), p)
    }

    pub fn tree_prefix(&self) -> String {
        String::from("tree")
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default, rename = "defaultLinkType")]
    pub default_link_type: LinkType,
    /// Ignore patterns, relative to ./tree/
    #[serde(default, rename = "ignorePatterns")]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileProcess>,
    #[serde(default, rename = "templateFiles")]
    pub template_files: Vec<TemplateProcess>,
    #[serde(default, rename = "beforeLink")]
    pub before_link: Vec<Hook>,
    #[serde(default, rename = "afterLink")]
    pub after_link: Vec<Hook>,
    #[serde(default = "default_replace_files", rename = "replaceFiles")]
    pub replace_files: bool,
    #[serde(default, rename = "replaceDirectories")]
    pub replace_directories: bool,
}

fn default_replace_files() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct FileProcess {
    /// Path of the file, relative to package root.
    pub src: String,
    /// Destination of linking, relative to tree root.
    pub dest: String,
    #[serde(default, rename = "linkType")]
    pub link_type: LinkType,
    #[serde(default, rename = "replaceFiles")]
    pub replace_files: Option<bool>,
    #[serde(default, rename = "replaceDirectories")]
    pub replace_directories: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub enum LinkType {
    Link,
    Copy,
}

impl Default for LinkType {
    fn default() -> Self {
        Self::Link
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplateProcess {
    /// Path to the template, relative to package root.
    pub src: String,
    /// Destination, relative to tree root.
    pub dest: String,
    #[serde(default)]
    pub engine: TemplateEngine,
    #[serde(default, rename = "replaceFiles")]
    pub replace_files: Option<bool>,
    #[serde(default, rename = "replaceDirectories")]
    pub replace_directories: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub enum TemplateEngine {
    Gtmpl,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::Gtmpl
    }
}

#[derive(Debug, Deserialize)]
pub struct Hook {
    pub string: String,
    pub name: String,
}
