use std::path::PathBuf;

pub mod package;
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
