use std::{collections::HashMap, path::PathBuf};

use crate::{
    package_format::PackageFormat, semver::SemVer, sources::git_src, ConnectionPolicy, NumngError,
};

pub type PackageId = usize;

/// Why is everytning optional? to allow merging
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: Option<String>,
    pub linkin: Option<HashMap<String, PackageId>>,
    pub path_offset: Option<String>,
    pub depends: Option<Vec<PackageId>>,
    pub package_format: Option<PackageFormat>,
    pub ignore_registry: Option<bool>,
    pub version: Option<SemVer>,

    pub nu_plugins: Option<Vec<String>>,
    pub nu_libs: Option<HashMap<String, String>>,
    pub shell_config: Option<HashMap<String, Vec<String>>>,
    pub bin: Option<HashMap<String, String>>,
    pub build_command: Option<String>,
    pub allow_build_commands: Option<bool>,

    pub source_type: Option<SourceType>,
    pub source_uri: Option<String>,
    pub git_ref: Option<String>,
    // when adding new values don't forget to update self.fill_null_values
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    Git,
}

impl Package {
    pub fn new_with_name(name: String) -> Self {
        Self {
            bin: None,
            build_command: None,
            depends: None,
            ignore_registry: None,
            linkin: None,
            name: Some(name),
            nu_libs: None,
            nu_plugins: None,
            package_format: None,
            path_offset: None,
            shell_config: None,
            version: None,
            allow_build_commands: None,
            source_type: None,
            source_uri: None,
            git_ref: None,
        }
    }
    pub fn new_empty() -> Self {
        Self {
            bin: None,
            build_command: None,
            depends: None,
            ignore_registry: None,
            linkin: None,
            name: None,
            nu_libs: None,
            nu_plugins: None,
            package_format: None,
            path_offset: None,
            shell_config: None,
            version: None,
            allow_build_commands: None,
            source_type: None,
            source_uri: None,
            git_ref: None,
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
        if self.source_type.is_none() {
            self.source_type = filler.source_type;
        }
        if self.source_uri.is_none() {
            self.source_uri = filler.source_uri;
        }
        if self.source_type.is_none() {
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
        connection_policy: &ConnectionPolicy,
    ) -> Result<PathBuf, NumngError> {
        let res = match &self.source_type {
            Some(SourceType::Git) | None => git_src::get_package_fs_basepath(
                &self
                    .source_uri
                    .clone()
                    .ok_or_else(|| NumngError::InvalidPackageFieldValue {
                        package_name: self.name.clone(),
                        field: String::from("source_uri"),
                        value: None,
                    })?,
                &self.git_ref.clone().unwrap_or(String::from("main")),
                base_dir,
                &connection_policy,
            )?,
        };
        Ok(match &self.path_offset {
            Some(path) => res.join(path),
            None => res,
        })
    }

    pub fn as_registry(
        &self,
        base_dir: &PathBuf,
        connection_policy: &ConnectionPolicy,
    ) -> Result<Box<dyn crate::repo::Repository>, NumngError> {
        match self.package_format {
            Some(PackageFormat::Numng) => Ok(Box::new(crate::repo::numng::NumngRepo::new(
                self.get_fs_basepath(base_dir, connection_policy)?,
            ))),
            Some(PackageFormat::Nupm) => {
                todo!("Nupm registry creation in package::Package::as_registry")
            }
            Some(PackageFormat::PackerNu) => {
                unimplemented!("PackerNu registry creation in package::Package::as_registry")
            }
            None => Err(NumngError::InvalidPackageFieldValue {
                package_name: self.name.clone(),
                field: String::from("package_format"),
                value: None,
            }),
        }
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Package(name={}, version={}, source_uri={}, git_ref={})",
            format_opt_str(self.name.clone()),
            (if let Some(v) = self.version.clone() {
                v
            } else {
                SemVer::Latest
            }),
            format_opt_str(self.source_uri.clone()),
            format_opt_str(self.git_ref.clone())
        )
    }
}

fn format_opt_str(os: Option<String>) -> String {
    if let Some(n) = os {
        n
    } else {
        String::from("None")
    }
}
