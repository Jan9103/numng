use std::{cmp::Ordering, fs::File, path::PathBuf};

use serde_json::Value;

use crate::{package::Package, semver::SemVer, NumngError};

pub struct NumngRepo {
    base_path: PathBuf,
}

impl NumngRepo {
    pub fn new(base_path: PathBuf) -> Self {
        log::trace!(
            "New NumngRepo @{}",
            base_path
                .as_os_str()
                .to_str()
                .expect("Failed to decode PathBuf to str (NumngRepo::new)")
        );
        Self { base_path }
    }
}

impl super::Repository for NumngRepo {
    /// return values:
    ///   Err: something went wrong
    ///   Ok(None): package(-version) not found
    ///   Ok(Some(Package)): here you go
    fn get_package(
        &self,
        collection: &mut crate::package::PackageCollection,
        name: &String,
        version: &crate::semver::SemVer,
    ) -> Result<Option<crate::package::Package>, crate::NumngError> {
        log::trace!("NumngRepo.get_package {} {}", name, version);
        let json_path: PathBuf = self.base_path.join(format!("{}.json", name));
        if !json_path.starts_with(&self.base_path) {
            return Err(crate::NumngError::SecurityError(format!(
                "Package-name escapes NumngRepo.base_path: {}",
                &name
            )));
        }
        if !json_path.is_file() {
            log::trace!("NumngRepo.get_package -> Not a file (None)");
            return Ok(None);
        }
        let file: File = match File::open(&json_path) {
            Ok(v) => v,
            Err(e) => {
                return Err(crate::NumngError::IoError(e));
            }
        };
        let json_value: serde_json::Value = match serde_json::from_reader(file) {
            Ok(v) => v,
            Err(e) => return Err(crate::NumngError::InvalidJsonError(e)),
        };
        let vers: Vec<(SemVer, Value)> = if let Value::Object(o) = json_value {
            o.into_iter()
                .map(|i| -> Result<(SemVer, Value), NumngError> {
                    Ok((SemVer::from_string(&i.0)?, i.1))
                })
                .collect::<Result<Vec<(SemVer, Value)>, NumngError>>()?
        } else {
            return Err(crate::NumngError::InvalidRegistryFormat(
                json_path,
                String::from("NumngRepo package-json does not have a record as root-element"),
            ));
        };
        let fallback_values: Option<Value> = match vers
            .iter()
            .find(|i| -> bool { matches!(i.0, SemVer::RegistryFallbackValues) })
        {
            Some(v) => Some(v.1.clone()),
            None => None,
        };
        let vers: Option<(SemVer, Value)> = vers
            .into_iter()
            .filter(|i| -> bool { version.matches(&i.0) })
            .max_by(|x, y| -> Ordering {
                if x.0.greater_than(&y.0) {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            });
        if let Some(v) = vers {
            let mut package: Package =
                crate::package::numng::parse_numng_package(collection, &v.1, None)?;
            if let Some(f) = fallback_values {
                let fbp: Package =
                    crate::package::numng::parse_numng_package(collection, &f, None)?;
                package.fill_null_values(fbp);
            }
            log::trace!("NumngRepo.get_package -> Found a match (Some)");
            dbg!(&package);
            Ok(Some(package))
        } else {
            log::trace!("NumngRepo.get_package -> No matching version (None)");
            Ok(None)
        }
    }
}
