use std::{os::unix::process::CommandExt, path::PathBuf, str::FromStr};

use crate::{
    package::{Package, PackageId},
    semver::SemVer,
    util::try_run_command,
    ConnectionPolicy, NumngError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PackageCollection {
    packages: Vec<Package>,
}

type Type = PackageId;

pub fn parse_numng_json(
    json_value: &serde_json::Value,
    base_dir: &PathBuf,
    connection_policy: &ConnectionPolicy,
    use_registry: bool,
    allow_build_commands: Option<bool>,
) -> Result<(PackageCollection, PackageId), NumngError> {
    let mut c: PackageCollection = PackageCollection::new();
    let pid: Type = c.append_numng_package_json(json_value, allow_build_commands)?;
    if use_registry {
        let repos: Vec<Box<dyn crate::repo::Repository>> =
            crate::package_format::numng::parse_repos_from_package(json_value)?
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
    ) -> Result<Type, NumngError> {
        let p: Package = crate::package_format::numng::parse_numng_package(
            self,
            package_json,
            allow_build_commands,
        )?;
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

    // not needed - it already has to be sorted based on how it is implemented ^^
    // pub fn sort_dependcies(&self) -> Result<Vec<PackageId>, NumngError> {
    //     let mut unsorted_packages: Vec<(PackageId, Vec<PackageId>)> = self
    //         .packages
    //         .iter()
    //         .enumerate()
    //         .map(|i| -> (PackageId, Vec<PackageId>) {
    //             let depends: Vec<PackageId> = i.1.depends.clone().unwrap_or(Vec::new());
    //             (i.0, depends)
    //         })
    //         .collect::<Vec<(PackageId, Vec<PackageId>)>>();
    //     let mut last_len: usize = unsorted_packages.len();
    //     let mut out: Vec<PackageId> = Vec::new();

    //     while last_len > 0 {
    //         // let m: Vec<PackageId> = unsorted_packages
    //         //     .drain_filter(|i| i.1.len() == 0)
    //         //     .collect::<Vec<PackageId>>();
    //         let mut m: Vec<PackageId> = Vec::new();
    //         unsorted_packages = unsorted_packages
    //             .into_iter()
    //             .filter_map(|i| {
    //                 if i.1.len() == 0 {
    //                     Some(i)
    //                 } else {
    //                     m.push(i.0);
    //                     None
    //                 }
    //             })
    //             .collect();
    //         unsorted_packages = unsorted_packages
    //             .into_iter()
    //             .map(|i| -> (PackageId, Vec<PackageId>) {
    //                 (i.0, i.1.into_iter().filter(|d| !m.contains(d)).collect())
    //             })
    //             .collect();
    //         out.append(&mut m);

    //         let tmp: usize = unsorted_packages.len();
    //         if tmp == last_len {
    //             let offending_packages: Vec<Package> = unsorted_packages
    //                 .iter()
    //                 .map(|i| -> Package { self.get_package(i.0).unwrap().clone() })
    //                 .collect::<Vec<Package>>();
    //             return Err(NumngError::CircularDependencies(offending_packages));
    //         }
    //         last_len = tmp;
    //     }
    //     Ok(out)
    // }

    pub fn build_environment(
        &self,
        base_dir: &PathBuf,
        nupm_home: &PathBuf,
        enable_script: Option<PathBuf>,
        enable_overlay: Option<PathBuf>,
        delete_existing_nupm_home: bool,
        connection_policy: &ConnectionPolicy,
        handle_nu_plugins: bool,
        allow_build_commands: Option<bool>,
    ) -> Result<(), NumngError> {
        log::info!("building environment..");
        const DEFAULT_ALLOW_BUILD_COMMANDS: bool = false;

        if nupm_home.exists() {
            log::trace!("nupm_home exists");
            if !delete_existing_nupm_home {
                return Err(NumngError::NupmHomeAlreadyExists(nupm_home.clone()));
            }
            std::fs::remove_dir_all(&nupm_home).map_err(|err| NumngError::IoError(err))?;
        }
        std::fs::create_dir_all(&nupm_home).map_err(|err| NumngError::IoError(err))?;

        let mut ls_o_use: Vec<String> = Vec::new();
        let mut ls_o_env: Vec<String> = Vec::new();
        let mut ls_s: Vec<String> = Vec::new();
        // TODO: avoid pulling updates for 1 git-branch twice if it is included twice (vec to pass along to `get_base_path`?)

        for package in self.packages.iter() {
            log::trace!("building package {}", package);
            let mut package: Package = package.clone();
            let package_base_path: PathBuf =
                package.get_fs_basepath(base_dir, connection_policy)?;

            // TODO: load in-package numng.json, nupm.nuon, etc

            if let Some(linkin) = &package.linkin {
                log::trace!("linkin present");
                for (a, linkin_package_id) in linkin {
                    let linkin_package: &Package = self
                        .get_package(*linkin_package_id)
                        .expect("Linkin package not in collection?!");
                    let linkin_package_path: PathBuf =
                        linkin_package.get_fs_basepath(base_dir, connection_policy)?;
                    let (local_path, in_package_path): (PathBuf, PathBuf) =
                        if let Some((a, b)) = a.split_once(":") {
                            (
                                package_base_path.join(PathBuf::from_str(a).map_err(|_| {
                                    NumngError::InvalidPackageFieldValue {
                                        package_name: package.name.clone(),
                                        field: String::from("linkin"),
                                        value: Some(String::from(a)),
                                    }
                                })?),
                                linkin_package_path.join(PathBuf::from_str(b).map_err(|_| {
                                    NumngError::InvalidPackageFieldValue {
                                        package_name: package.name.clone(),
                                        field: String::from("linkin"),
                                        value: Some(String::from(a)),
                                    }
                                })?),
                            )
                        } else {
                            (
                                package_base_path.join(PathBuf::from_str(a).map_err(|_| {
                                    NumngError::InvalidPackageFieldValue {
                                        package_name: package.name.clone(),
                                        field: String::from("linkin"),
                                        value: Some(String::from(a)),
                                    }
                                })?),
                                linkin_package_path,
                            )
                        };
                    crate::util::symlink(&in_package_path, &local_path)?;
                }
            }

            if let Some(build_command) = &package.build_command {
                log::trace!("build_command present: {}", build_command);
                if !allow_build_commands
                    .or(allow_build_commands)
                    .unwrap_or(DEFAULT_ALLOW_BUILD_COMMANDS)
                {
                    return Err(NumngError::BuildCommandBlocked(package.clone()));
                }
                match build_command.as_str() {
                    "cargo build --release" => {
                        try_run_command(
                            &mut std::process::Command::new("cargo")
                                .arg("build")
                                .arg("--release")
                                .arg("--quiet")
                                .current_dir(&package_base_path),
                        )?;
                    }
                    other => {
                        try_run_command(
                            std::process::Command::new("nu")
                                .arg("--log-level")
                                .arg("trace")
                                .arg("--no-history")
                                .arg("--no-config-file")
                                .arg("--commands")
                                .arg(other)
                                .current_dir(&package_base_path),
                        )?;
                        // TODO: run with nu
                    }
                }
            }
        }

        Ok(()) // TODO
    }
}
