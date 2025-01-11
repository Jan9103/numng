use crate::NumngError;

pub mod numng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageFormat {
    Numng,
    Nupm,
    PackerNu,
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
