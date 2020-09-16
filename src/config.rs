use crate::map::Map;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

type IgnorePatterns = Vec<String>;

#[derive(Debug, Deserialize)]
pub struct Package {
    #[serde(default)]
    pub variables: Map,
    #[serde(rename = "package")]
    pub config: Config,
}

impl Package {
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        for ext in &["dhall", "yaml", "json"] {
            let config_path = path.as_ref().join(format!("package.{}", ext));

            let file = match File::open(&config_path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let result = match *ext {
                "dhall" => Package::from_dhall_reader(&file),
                "yaml" => Package::from_yaml_reader(&file),
                "json" => Package::from_json_reader(&file),
                _ => panic!(),
            };

            let package = result.with_context(|| {
                format!(
                    "Failed to load package configuration from {}",
                    config_path.display()
                )
            })?;

            return Ok(package);
        }

        Err(anyhow!("Failed to read package configuration"))
    }

    fn from_dhall_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        let parsed = serde_dhall::from_str(&s).parse()?;
        Ok(parsed)
    }

    fn from_json_reader<R: Read>(reader: R) -> Result<Self> {
        let parsed = serde_json::from_reader(reader)?;
        Ok(parsed)
    }

    fn from_yaml_reader<R: Read>(reader: R) -> Result<Self> {
        let parsed = serde_yaml::from_reader(reader)?;
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Name of the package used in logging.
    pub name: String,
    /// Relative paths to dependencies.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Default link type across trees.
    #[serde(default, rename = "defaultLinkType")]
    pub default_link_type: LinkType,
    /// Global ignore patterns, relative to tree roots.
    #[serde(default, rename = "ignorePatterns")]
    pub ignore_patterns: IgnorePatterns,
    /// List of specific files to link differently from rest.
    #[serde(default)]
    pub files: Vec<FileProcess>,
    /// List of specific templates to process and write.
    #[serde(default, rename = "templateFiles")]
    pub template_files: Vec<TemplateProcess>,
    /// List of hooks to run before linking any files and templates.
    #[serde(default, rename = "beforeLink")]
    pub before_link: Vec<Hook>,
    /// List of hooks to run after linking any files and templates.
    #[serde(default, rename = "afterLink")]
    pub after_link: Vec<Hook>,
    /// Flag to replace existing files when linking.
    #[serde(default = "default_replace_files", rename = "replaceFiles")]
    pub replace_files: bool,
    /// Flag to replace existing directories when linking.
    #[serde(default, rename = "replaceDirectories")]
    pub replace_directories: bool,
    /// Tree configurations.
    #[serde(default = "default_trees", rename = "treePath")]
    pub trees: Vec<Tree>,
}

fn default_replace_files() -> bool {
    true
}

fn default_trees() -> Vec<Tree> {
    vec![Tree {
        path: String::from("tree"),
        default_link_type: None,
        ignore_patterns: vec![],
        replace_files: None,
        replace_directories: None,
    }]
}

#[derive(Debug, Deserialize)]
pub struct Tree {
    /// Relative path, from package root, to tree root.
    pub path: String,
    /// Default link type across trees.
    #[serde(default, rename = "defaultLinkType")]
    /// Ignore patterns, relative to tree roots.
    pub default_link_type: Option<LinkType>,
    #[serde(default, rename = "ignorePatterns")]
    pub ignore_patterns: IgnorePatterns,
    /// Flag to replace existing files when linking.
    #[serde(default, rename = "replaceFiles")]
    pub replace_files: Option<bool>,
    /// Flag to replace existing directories when linking.
    #[serde(default, rename = "replaceDirectories")]
    pub replace_directories: Option<bool>,
}

impl Tree {
    pub fn file_path_str(&self, s: &str) -> String {
        let p = if s.starts_with("/") {
            s.chars().skip(1).collect()
        } else {
            String::from(s)
        };

        format!("{}/{}", self.path, p)
    }

    /// Get the path of some path (relative to the tree root) relative to the project root.
    pub fn file_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.path_buf().join(path)
    }

    pub fn path_buf(&self) -> PathBuf {
        self.path.clone().into()
    }
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
