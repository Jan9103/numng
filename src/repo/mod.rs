use crate::{package::Package, semver::SemVer, NumngError};

pub mod numng;

pub trait Repository {
    fn get_package(
        &self,
        collection: &mut crate::PackageCollection,
        name: &String,
        version: &SemVer,
    ) -> Result<Option<Package>, NumngError>;
}
