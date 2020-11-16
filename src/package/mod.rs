mod map;
pub use map::{Map, Value as MapValue};

use std::path::PathBuf;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Package {
    pub name: String,
    pub dependencies: Vec<String>,
    pub files: PackageFiles,
    pub templates: PackageTemplates,
    pub hooks: PackageHooks,
    pub variables: Map,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackageFiles {
    pub trees: Vec<Tree>,
    pub extra: Vec<File>,
    pub link_type: LinkType,
    pub ignore: Vec<String>,
    pub replace_files: bool,
    pub replace_dirs: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackageTemplates {
    pub templates: Vec<Template>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackageHooks {
    pub pre: Vec<Hook>,
    pub post: Vec<Hook>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Tree {
    pub path: PathBuf,
    pub link_type: LinkType,
    pub ignore: Vec<String>,
    pub replace_files: bool,
    pub replace_dirs: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct File {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub link_type: LinkType,
    pub replace_files: bool,
    pub replace_dirs: bool,
}

#[derive(Clone, Debug, Serialize)]
pub enum LinkType {
    Link,
    Copy,
}

#[derive(Clone, Debug, Serialize)]
pub struct Template {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub ty: TemplateType,
    pub replace_files: bool,
    pub replace_dirs: bool,
}

#[derive(Clone, Debug, Serialize)]
pub enum TemplateType {
    Handlebars { partials: Vec<(String, PathBuf)> },
    Gotmpl,
    Tera,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hook {
    pub name: String,
    pub ty: HookType,
}

#[derive(Clone, Debug, Serialize)]
pub enum HookType {
    Executable { path: PathBuf },
    LuaFunction { name: String },
}
