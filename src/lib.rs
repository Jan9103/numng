use std::path::PathBuf;

// pub mod commands;
pub mod package;
mod package_collection;
pub mod package_format;
pub mod repo;
pub mod semver;
pub mod sources;
pub mod util;
pub use package_collection::{parse_numng_json, PackageCollection};
mod numng_error;
pub use numng_error::NumngError;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionPolicy {
    Offline,
    Download,
    Update,
}

impl std::fmt::Display for ConnectionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Offline => "offline",
                Self::Download => "download",
                Self::Update => "update",
            }
        )
    }
}
