use std::{collections::HashMap, path::PathBuf};

use crate::NumngError;

mod git_src;
pub mod numng;

#[allow(dead_code)]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Package {
    name: Option<String>,
    linkin: Option<HashMap<String, Box<Package>>>,
    source_type: Option<String>,
    source_uri: Option<String>,
    git_ref: Option<String>,
    path_offset: Option<String>,
    depends: Option<Vec<Box<Package>>>,
    package_format: Option<String>,
    ignore_registry: Option<bool>,
    version: Option<String>,

    nu_plugins: Option<Vec<String>>,
    registry: Option<Vec<Box<Package>>>,
    nu_libs: Option<HashMap<String, String>>,
    shell_config: Option<HashMap<String, Vec<String>>>,
    bin: Option<HashMap<String, String>>,
    build_command: Option<String>,
    allow_build_commands: Option<bool>,
    // when adding new values don't forget to update self.fill_null_values
}

impl Package {
    pub fn new_empty() -> Self {
        Self {
            allow_build_commands: None,
            bin: None,
            build_command: None,
            depends: None,
            git_ref: None,
            ignore_registry: None,
            linkin: None,
            name: None,
            nu_libs: None,
            nu_plugins: None,
            package_format: None,
            path_offset: None,
            registry: None,
            shell_config: None,
            source_type: None,
            source_uri: None,
            version: None,
        }
    }

    /// intended for filling in from a registry
    /// intentionally does not fill: "allow_build_commands" and "registry"
    pub fn fill_null_values(&mut self, filler: Package) {
        if self.name.is_none() {
            self.name = filler.name;
        }
        if self.linkin.is_none() {
            self.linkin = filler.linkin;
        }
        if self.source_uri.is_none() {
            self.source_type = filler.source_type;
            self.source_uri = filler.source_uri;
        }
        if self.git_ref.is_none() {
            self.git_ref = filler.git_ref;
        }
        if self.path_offset.is_none() {
            self.path_offset = filler.path_offset;
        }
        if self.depends.is_none() {
            self.depends = filler.depends;
        }
        if self.package_format.is_none() {
            self.package_format = filler.package_format;
        }
        if self.version.is_none() {
            self.version = filler.version;
        }
        if self.nu_plugins.is_none() {
            self.nu_plugins = filler.nu_plugins;
        }
        if self.nu_libs.is_none() {
            self.nu_libs = filler.nu_libs;
        }
        if self.shell_config.is_none() {
            self.shell_config = filler.shell_config;
        }
        if self.bin.is_none() {
            self.bin = filler.bin;
        }
        if self.build_command.is_none() {
            self.build_command = filler.build_command;
        }
    }

    pub fn get_fs_basepath(
        &self,
        base_dir: &PathBuf,
        connection_policy: ConnectionPolicy,
    ) -> Result<PathBuf, NumngError> {
        match self
            .source_type
            .clone()
            .unwrap_or(String::from("git"))
            .as_str()
        {
            "git" => git_src::get_package_fs_basepath(self, base_dir, connection_policy),
            _ => Err(NumngError::InvalidPackageFieldValue {
                package_name: self.name.clone(),
                field: String::from("source_type"),
                value: self.source_type.clone(),
            }),
        }
    }
}

#[derive(PartialEq)]
pub enum ConnectionPolicy {
    Offline,
    Download,
    Update,
}
