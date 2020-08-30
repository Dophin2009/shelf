use crate::map::Map;

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub variables: Map,
    pub package: Package,
}

impl Config {
    pub fn from_dhall_file<P: AsRef<Path>>(path: P) -> serde_dhall::Result<Self> {
        serde_dhall::from_file(path.as_ref()).parse()
    }
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default, rename = "defaultLinkType")]
    pub default_link_type: LinkType,
    #[serde(default, rename = "ignorePatterns")]
    pub ignore_pattners: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileProcess>,
    #[serde(default, rename = "templateFiles")]
    pub template_files: Vec<TemplateProcess>,
    #[serde(default, rename = "beforeLink")]
    pub before_link: Vec<Hook>,
    #[serde(default, rename = "afterLink")]
    pub after_link: Vec<Hook>,
}

#[derive(Debug, Deserialize)]
pub struct FileProcess {
    pub path: String,
    #[serde(default, rename = "linkType")]
    pub link_type: LinkType,
    #[serde(default)]
    pub shallow: bool,
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
    pub src: String,
    pub dest: String,
    #[serde(default)]
    pub engine: TemplateEngine,
}

#[derive(Debug, Deserialize)]
pub enum TemplateEngine {
    Liquid,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::Liquid
    }
}

#[derive(Debug, Deserialize)]
pub struct Hook {
    pub string: String,
    pub name: String,
}
