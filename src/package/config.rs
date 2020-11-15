use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Name of the package used in logging.
    pub name: String,
    /// Relative paths to dependencies.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Default link type across trees.
    #[serde(default, rename = "link_type")]
    pub default_link_type: LinkType,
    /// Global ignore patterns, relative to tree roots.
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    /// List of specific files to link differently from rest.
    #[serde(default)]
    pub files: Vec<FileProcess>,
    /// List of specific templates to process and write.
    #[serde(default, rename = "templates")]
    pub template_files: Vec<TemplateProcess>,
    /// List of hooks to run before linking any files and templates.
    #[serde(default)]
    pub before_link: Vec<Hook>,
    /// List of hooks to run after linking any files and templates.
    #[serde(default)]
    pub after_link: Vec<Hook>,
    /// Flag to replace existing files when linking.
    #[serde(default = "Config::default_replace_files")]
    pub replace_files: bool,
    /// Flag to replace existing directories when linking.
    #[serde(default, rename = "replace_dirs")]
    pub replace_directories: bool,
    /// Tree configurations.
    #[serde(default = "Config::default_trees")]
    pub trees: Vec<Tree>,
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn new_optional(
        name: String,
        dependencies: Option<Vec<String>>,
        default_link_type: Option<LinkType>,
        ignore_patterns: Option<Vec<String>>,
        files: Option<Vec<FileProcess>>,
        template_files: Option<Vec<TemplateProcess>>,
        before_link: Option<Vec<Hook>>,
        after_link: Option<Vec<Hook>>,
        replace_file: Option<bool>,
        replace_directories: Option<bool>,
        trees: Option<Vec<Tree>>,
    ) -> Self {
        Self {
            name,
            dependencies: dependencies.unwrap_or_default(),
            default_link_type: default_link_type.unwrap_or_default(),
            ignore_patterns: ignore_patterns.unwrap_or_default(),
            files: files.unwrap_or_default(),
            template_files: template_files.unwrap_or_default(),
            before_link: before_link.unwrap_or_default(),
            after_link: after_link.unwrap_or_default(),
            replace_files: replace_file.unwrap_or_else(Self::default_replace_files),
            replace_directories: replace_directories.unwrap_or_default(),
            trees: trees.unwrap_or_else(Self::default_trees),
        }
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tree {
    /// Relative path, from package root, to tree root.
    pub path: String,
    /// Default link type across trees.
    #[serde(default, rename = "link_type")]
    /// Ignore patterns, relative to tree roots.
    pub default_link_type: Option<LinkType>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    /// Flag to replace existing files when linking.
    #[serde(default)]
    pub replace_files: Option<bool>,
    /// Flag to replace existing directories when linking.
    #[serde(default, rename = "replace_dirs")]
    pub replace_directories: Option<bool>,
}

impl Tree {
    pub fn new_optional(
        path: String,
        default_link_type: Option<LinkType>,
        ignore_patterns: Option<Vec<String>>,
        replace_files: Option<bool>,
        replace_directories: Option<bool>,
    ) -> Self {
        Self {
            path,
            default_link_type,
            ignore_patterns: ignore_patterns.unwrap_or_default(),
            replace_files,
            replace_directories,
        }
    }

    pub fn file_path_str(&self, s: &str) -> String {
        let p = if s.starts_with('/') {
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

#[derive(Clone, Debug, Deserialize)]
pub struct FileProcess {
    /// Path of the file, relative to package root.
    pub src: String,
    /// Destination of linking, relative to tree root.
    pub dest: String,
    #[serde(default)]
    pub link_type: LinkType,
    #[serde(default)]
    pub replace_files: Option<bool>,
    #[serde(default, rename = "replace_dirs")]
    pub replace_directories: Option<bool>,
}

impl FileProcess {
    pub fn new_optional(
        src: String,
        dest: String,
        link_type: Option<LinkType>,
        replace_files: Option<bool>,
        replace_directories: Option<bool>,
    ) -> Self {
        Self {
            src,
            dest,
            link_type: link_type.unwrap_or_default(),
            replace_files,
            replace_directories,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum LinkType {
    Link,
    Copy,
}

impl Default for LinkType {
    fn default() -> Self {
        Self::Link
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TemplateProcess {
    /// Path to the template, relative to package root.
    pub src: String,
    /// Destination, relative to tree root.
    pub dest: String,
    #[serde(default)]
    pub engine: TemplateEngine,
    #[serde(default)]
    pub replace_files: Option<bool>,
    #[serde(default, rename = "replace_dirs")]
    pub replace_directories: Option<bool>,
}

impl TemplateProcess {
    pub fn new_optional(
        src: String,
        dest: String,
        engine: Option<TemplateEngine>,
        replace_files: Option<bool>,
        replace_directories: Option<bool>,
    ) -> Self {
        Self {
            src,
            dest,
            engine: engine.unwrap_or_default(),
            replace_files,
            replace_directories,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum TemplateEngine {
    Gtmpl,
    Tera,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::Gtmpl
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Hook {
    pub string: String,
    pub name: String,
}

impl Hook {
    pub fn new(name: String, string: String) -> Self {
        Self { string, name }
    }
}
