use std::path::PathBuf;

// pub mod commands;
pub mod package;
// pub mod packlist;
pub mod repo;
pub mod semver;
pub mod util;

pub fn hello_world() {
    println!("Hello World");
}

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
}

pub fn get_base_directory() -> PathBuf {
    // according to <https://github.com/rust-lang/cargo/tree/master/crates/home>
    // this is not actually deprecated and still the recommended method..
    #[allow(deprecated)]
    let home_dir: PathBuf = std::env::home_dir().expect(
        "Rust can't find your home directory. How did you even compile this for templeos??",
    );

    home_dir
        .join(".local")
        .join("share")
        .join("nushell")
        .join("numng")
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
                    "Package definition for {} is fault. field {} is null",
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
        }
    }
}
