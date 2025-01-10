use std::{collections::HashMap, path::PathBuf};

use crate::{semver::SemVer, NumngError};

mod git_src;
pub mod numng;

pub type PackageId = usize;

/// Why is everytning optional? to allow merging
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    name: Option<String>,
    linkin: Option<HashMap<String, PackageId>>,
    path_offset: Option<String>,
    depends: Option<Vec<PackageId>>,
    package_format: Option<PackageFormat>,
    ignore_registry: Option<bool>,
    version: Option<SemVer>,

    nu_plugins: Option<Vec<String>>,
    // registry: Option<Vec<Box<Package>>>,
    nu_libs: Option<HashMap<String, String>>,
    shell_config: Option<HashMap<String, Vec<String>>>,
    bin: Option<HashMap<String, String>>,
    build_command: Option<String>,
    allow_build_commands: Option<bool>,

    source_type: Option<SourceType>,
    source_uri: Option<String>,
    git_ref: Option<String>,
    // when adding new values don't forget to update self.fill_null_values
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    Git,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageFormat {
    Numng,
    Nupm,
    PackerNu,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageCollection {
    packages: Vec<Package>,
}

pub fn parse_numng_json(
    json_value: &serde_json::Value,
    base_dir: &PathBuf,
    connection_policy: &ConnectionPolicy,
    use_registry: bool,
    allow_build_commands: Option<bool>,
) -> Result<(PackageCollection, PackageId), NumngError> {
    let mut c: PackageCollection = PackageCollection::new();
    let pid: PackageId = c.append_numng_package_json(json_value, allow_build_commands)?;
    if use_registry {
        let repos: Vec<Box<dyn crate::repo::Repository>> =
            numng::parse_repos_from_package(json_value)?
                .into_iter()
                .map(
                    |i| -> Result<Box<dyn crate::repo::Repository>, NumngError> {
                        Ok(i.as_registry(base_dir, connection_policy)?)
                    },
                )
                .collect::<Result<Vec<Box<dyn crate::repo::Repository>>, NumngError>>()?;
        for registry in repos.iter() {
            c.apply_registry(registry)?;
        }
    }
    Ok((c, pid))
}

impl PackageCollection {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
        }
    }
    pub fn append_numng_package_json(
        &mut self,
        package_json: &serde_json::Value,
        allow_build_commands: Option<bool>,
    ) -> Result<PackageId, NumngError> {
        let p: Package = numng::parse_numng_package(self, package_json, allow_build_commands)?;
        Ok(self.append_package(p))
    }

    pub fn append_package(&mut self, package: Package) -> PackageId {
        self.packages.push(package);
        self.packages.len() - 1
    }

    pub fn get_package(&self, package_id: PackageId) -> Option<&Package> {
        self.packages.get(package_id)
    }

    pub fn apply_registry(
        &mut self,
        registry: &Box<dyn crate::repo::Repository>,
    ) -> Result<(), NumngError> {
        let packages_to_search: Vec<Option<(String, SemVer)>> = self
            .packages
            .iter()
            .map(|i| -> Option<(String, SemVer)> {
                if let Some(pn) = i.name.clone() {
                    Some((pn.clone(), i.version.clone().unwrap_or(SemVer::Latest)))
                } else {
                    None
                }
            })
            .collect();
        let registry_packages: Vec<Option<Package>> = packages_to_search
            .into_iter()
            .map(|i| -> Result<Option<Package>, NumngError> {
                Ok(if let Some((pn, sv)) = i {
                    registry.get_package(self, &pn, &sv)?
                } else {
                    None
                })
            })
            .collect::<Result<Vec<Option<Package>>, NumngError>>()?;
        registry_packages.into_iter().enumerate().for_each(|it| {
            if let Some(registry_package) = it.1 {
                if let Some(p) = self.packages.get_mut(it.0) {
                    if !matches!(p.ignore_registry, Some(true)) {
                        p.fill_null_values(registry_package);
                    }
                }
            }
        });
        Ok(())
    }
}

impl PackageFormat {
    pub fn from_string(package_name: &Option<String>, s: &str) -> Result<Self, NumngError> {
        Ok(match s.to_lowercase().as_str() {
            "numng" => Self::Numng,
            "nupm" => Self::Nupm,
            "packer.nu" | "packer" => Self::PackerNu,
            o => {
                return Err(NumngError::InvalidPackageFieldValue {
                    package_name: package_name.clone(),
                    field: String::from("package_format"),
                    value: Some(String::from(o)),
                })
            }
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionPolicy {
    Offline,
    Download,
    Update,
}
