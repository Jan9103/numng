use super::Package;
use serde_json::Value;
use std::collections::HashMap;

use crate::{semver::SemVer, NumngError};

use super::PackageCollection;
use super::PackageId;

const VALID_SHELL_CONFIG_KEYS: &[&str] = &["source", "use", "use_all", "source_env"];

pub fn parse_repos_from_package(json_value: &Value) -> Result<Vec<Package>, NumngError> {
    log::trace!("[parse_repos_from_package] start");
    let mut c: PackageCollection = PackageCollection::new();
    Ok(match json_value.get("registry") {
        Some(v) => match v {
            Value::Object(_) => vec![parse_numng_package(&mut c, v, Some(false))?],
            Value::Array(a) => a
                .into_iter()
                .map(|i| -> Result<Package, NumngError> {
                    parse_numng_package(&mut c, i, Some(false))
                })
                .collect::<Result<Vec<Package>, NumngError>>()?,
            o => {
                return Err(NumngError::InvalidPackageFieldValue {
                    package_name: None,
                    field: String::from("registry"),
                    value: Some(format!("{:?}", o)),
                })
            }
        },
        None => vec![],
    }
    .into_iter()
    .map(|mut i| -> Package {
        i.depends = None;
        i.linkin = None;
        i
    })
    .collect())
}

pub fn parse_numng_package(
    collection: &mut PackageCollection,
    json_value: &Value,
    allow_build_commands: Option<bool>,
) -> Result<Package, NumngError> {
    let name: Option<String> = json_get_opt_str(&None, &json_value, "name")?;
    let allow_build_commands: Option<bool> = if matches!(allow_build_commands, Some(false)) {
        Some(false)
    } else {
        json_get_opt_bool(&name, json_value, "allow_build_commands")?
    };
    let linkin: Option<HashMap<String, PackageId>> = match json_value.get("linkin") {
        Some(Value::Object(v)) => Some(
            v.into_iter()
                .map(|i| -> Result<(String, PackageId), NumngError> {
                    let linkin_path: String = i.0.clone();
                    match i.1 {
                        Value::Object(_) => Ok((
                            linkin_path,
                            collection
                                .append_numng_package_json(i.1, allow_build_commands.clone())?,
                        )),
                        _ => Err(NumngError::InvalidPackageFieldValue {
                            package_name: name.clone(),
                            field: format!("linkin ({})", linkin_path),
                            value: Some(String::from("Value not a record (package)")),
                        }),
                    }
                })
                .collect::<Result<HashMap<String, PackageId>, NumngError>>()?,
        ),
        Some(v) => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: name,
                field: String::from("linkin"),
                value: Some(format!("{:?}", v)),
            });
        }
        None => None,
    };
    let source_type: Option<super::SourceType> = match json_value.get("source_type") {
        Some(Value::String(v)) if v.as_str() == "git" => Some(super::SourceType::Git),
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: name,
                field: String::from("source_type"),
                value: Some(format!("{:?}", o)),
            });
        }
    };
    let git_ref: Option<String> = json_get_opt_str(&name, json_value, "git_ref")?;
    let source_uri: Option<String> = json_get_opt_str(&name, json_value, "source_uri")?;
    let path_offset: Option<String> = json_get_opt_str(&name, json_value, "path_offset")?;
    let package_format: Option<super::PackageFormat> =
        match json_get_opt_str(&name, json_value, "package_format")? {
            Some(v) => Some(super::PackageFormat::from_string(&name, v.as_str())?),
            None => None,
        };
    let ignore_registry: Option<bool> = json_get_opt_bool(&name, json_value, "ignore_registry")?;
    let depends: Option<Vec<PackageId>> = match json_value.get("depends") {
        Some(Value::Array(a)) => Some(
            a.into_iter()
                .map(|i| -> Result<PackageId, NumngError> {
                    match i {
                        Value::String(s) => {
                            Ok(collection.append_package(Package::new_with_name(String::from(s))))
                        }
                        Value::Object(_) => Ok(collection
                            .append_numng_package_json(&i, allow_build_commands.clone())?),
                        o => Err(NumngError::InvalidPackageFieldValue {
                            package_name: name.clone(),
                            field: String::from("depends"),
                            value: Some(format!("{:?}", o)),
                        }),
                    }
                })
                .collect::<Result<Vec<PackageId>, NumngError>>()?,
        ),
        Some(Value::String(s)) => Some(vec![
            collection.append_package(Package::new_with_name(String::from(s)))
        ]),
        Some(o) if matches!(o, Value::Object(_)) => {
            Some(vec![collection.append_numng_package_json(
                &o,
                allow_build_commands.clone(),
            )?])
        }
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: name,
                field: String::from("depends"),
                value: Some(format!("{:?}", o)),
            })
        }
    };
    let version: Option<SemVer> = match json_get_opt_str(&name, json_value, "version")? {
        Some(v) => Some(SemVer::from_string(&v)?),
        None => None,
    };
    let nu_plugins: Option<Vec<String>> = match json_value.get("nu_plugins") {
        Some(Value::Array(v)) => Some(
            v.into_iter()
                .map(|i| -> Result<String, NumngError> {
                    match i {
                        Value::String(s) => Ok(String::from(s)),
                        o => Err(NumngError::InvalidPackageFieldValue {
                            package_name: name.clone(),
                            field: String::from("nu_plugins"),
                            value: Some(format!("{:?}", o)),
                        }),
                    }
                })
                .collect::<Result<Vec<String>, NumngError>>()?,
        ),
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: name,
                field: String::from("nu_plugins"),
                value: Some(format!("{:?}", o)),
            })
        }
    };
    let nu_libs: Option<HashMap<String, String>> =
        json_get_opt_hm_str_str(&name, json_value, "nu_libs")?;
    let bin: Option<HashMap<String, String>> = json_get_opt_hm_str_str(&name, json_value, "bin")?;
    let build_command: Option<String> = json_get_opt_str(&name, json_value, "build_command")?;
    let shell_config: Option<HashMap<String, Vec<String>>> = match json_value.get("shell_config") {
        Some(Value::Object(o)) => {
            if !o
                .iter()
                .all(|i| VALID_SHELL_CONFIG_KEYS.contains(&i.0.as_str()))
            {
                return Err(NumngError::InvalidPackageFieldValue {
                    package_name: name,
                    field: String::from("shell_config"),
                    value: Some(String::from("invalid key")),
                });
            }
            Some(
                o.into_iter()
                    .map(|i| -> Result<(String, Vec<String>), NumngError> {
                        match i.1 {
                            Value::String(s) => Ok((i.0.clone(), vec![s.clone()])),
                            Value::Array(a) => Ok((
                                i.0.clone(),
                                a.into_iter()
                                    .map(|i2| -> Result<String, NumngError> {
                                        match i2 {
                                            Value::String(s) => Ok(s.clone()),
                                            o => Err(NumngError::InvalidPackageFieldValue {
                                                package_name: name.clone(),
                                                field: String::from("shell_config"),
                                                value: Some(format!("{:?}", o)),
                                            }),
                                        }
                                    })
                                    .collect::<Result<Vec<String>, NumngError>>()?,
                            )),
                            o => {
                                return Err(NumngError::InvalidPackageFieldValue {
                                    package_name: name.clone(),
                                    field: String::from("shell_config"),
                                    value: Some(format!("{:?}", o)),
                                })
                            }
                        }
                    })
                    .collect::<Result<HashMap<String, Vec<String>>, NumngError>>()?,
            )
        }
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: name,
                field: String::from("shell_config"),
                value: Some(format!("{:?}", o)),
            })
        }
    };

    Ok(super::Package {
        name,
        linkin,
        path_offset,
        depends,
        package_format,
        ignore_registry,
        version,
        nu_plugins,
        nu_libs,
        source_type,
        source_uri,
        git_ref,
        bin,
        build_command,
        allow_build_commands,
        shell_config,
    })
}

fn json_get_opt_str(
    package_name: &Option<String>,
    json_value: &serde_json::Value,
    key: &str,
) -> Result<Option<String>, NumngError> {
    match json_value.get(key) {
        Some(v) => match v.as_str() {
            Some(s) => Ok(Some(String::from(s))),
            None => Err(NumngError::InvalidPackageFieldValue {
                package_name: package_name.clone(),
                field: String::from(key),
                value: Some(format!("{:?}", v)),
            }),
        },
        None => Ok(None),
    }
}

fn json_get_opt_hm_str_str(
    package_name: &Option<String>,
    json_value: &Value,
    key: &str,
) -> Result<Option<HashMap<String, String>>, NumngError> {
    Ok(match json_value.get(key) {
        Some(Value::Object(o)) => Some(
            o.into_iter()
                .map(|i| -> Result<(String, String), NumngError> {
                    match i.1 {
                        Value::String(v) => Ok((i.0.clone(), v.clone())),
                        o => {
                            return Err(NumngError::InvalidPackageFieldValue {
                                package_name: package_name.clone(),
                                field: String::from(key),
                                value: Some(format!("{:?}", o)),
                            })
                        }
                    }
                })
                .collect::<Result<HashMap<String, String>, NumngError>>()?,
        ),
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: package_name.clone(),
                field: String::from(key),
                value: Some(format!("{:?}", o)),
            })
        }
    })
}

fn json_get_opt_bool(
    package_name: &Option<String>,
    json_value: &Value,
    key: &str,
) -> Result<Option<bool>, NumngError> {
    Ok(match json_value.get(key) {
        Some(Value::Bool(v)) => Some(*v),
        None => None,
        o => {
            return Err(NumngError::InvalidPackageFieldValue {
                package_name: package_name.clone(),
                field: String::from(key),
                value: Some(format!("{:?}", o)),
            })
        }
    })
}
