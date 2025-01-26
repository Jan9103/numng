use std::path::PathBuf;

use crate::package::Package;

#[derive(Debug)]
pub enum NumngError {
    ExternalCommandIO(std::io::Error),
    ExternalCommandExitcode {
        command: String,
        stdout: String,
        stderr: String,
        exitcode: i32,
    },
    InvalidPackageFieldValue {
        package_name: Option<String>,
        field: String,
        value: Option<String>,
    },
    NotImplemented(String),
    SecurityError(String),
    IoError(std::io::Error),
    InvalidSemVer {
        semver: String,
        issue: String,
    },
    InvalidJsonError(serde_json::Error),
    InvalidRegistryFormat(PathBuf, String),
    UnableToFetchResourceInOfflineMode(String),
    NupmHomeAlreadyExists(PathBuf),
    BuildCommandBlocked(Package),
    CircularDependencies(Vec<Package>),
}

impl std::fmt::Display for NumngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumngError::ExternalCommandIO(error) => {
                write!(f, "External command failed due to a IO error: {}", error)
            }
            NumngError::ExternalCommandExitcode {
                command,
                stdout,
                stderr,
                exitcode,
            } => write!(
                f,
                "External command ({}) failed (exitcode: {}). STDOUT:\n\n{}\n\nSTDERR:\n\n{}\n\n",
                command, exitcode, stdout, stderr
            ),
            NumngError::InvalidPackageFieldValue {
                package_name,
                field,
                value,
            } => match value {
                Some(v) => write!(
                    f,
                    "Package definition for {} is faulty. field {} contains value {}",
                    package_name
                        .clone()
                        .unwrap_or(String::from("<unknown name>")),
                    field,
                    v
                ),
                None => write!(
                    f,
                    "Package definition for {} is faulty. field {} is null",
                    package_name
                        .clone()
                        .unwrap_or(String::from("<unknown name>")),
                    field
                ),
            },
            NumngError::NotImplemented(s) => write!(f, "Feature not yet implemented: {}", s),
            NumngError::SecurityError(s) => {
                write!(f, "A potential security issue or bug was spotted: {}", s)
            }
            NumngError::IoError(error) => write!(f, "An IO-Error occured: {}", error),
            NumngError::InvalidSemVer { semver, issue } => {
                write!(f, "Invalid semantic version: {} ({})", semver, issue)
            }
            NumngError::InvalidJsonError(error) => write!(f, "Failed to parse json: {}", error),
            NumngError::InvalidRegistryFormat(path_buf, s) => write!(
                f,
                "A registry does not follow the specification. location: {}; problem: {}",
                path_buf
                    .as_os_str()
                    .to_str()
                    .unwrap_or("<error during error-rendering: failed to convert path to str>"),
                s
            ),
            NumngError::UnableToFetchResourceInOfflineMode(error) => {
                write!(f, "Unable to complete operating in offline mode: {}", error)
            }
            NumngError::NupmHomeAlreadyExists(path_buf) => write!(
                f,
                "Nupm Home already exists and overwriting is disabled: {}",
                path_buf
                    .to_str()
                    .expect("Failed to convert path_buf to str (NumngError::fmt)")
            ),
            NumngError::BuildCommandBlocked(package) => write!(
                f,
                "Unable to build package (Build commands disabled): {}",
                package
            ),
            NumngError::CircularDependencies(packages) => write!(
                f,
                "Failed to build (circular dependencies?). Packages, which can't be built: {}",
                packages
                    .iter()
                    .map(|i| -> String { format!("{}", i) })
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
        }
    }
}
