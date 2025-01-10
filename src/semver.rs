use crate::NumngError;

// more than 65535 major/minor/patch releases seem unlikely - can bump it if necesary
type SVNum = u16;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SemVer {
    Custom(String),
    Latest,
    Normal {
        major: SVNum,
        minor: Option<SVNum>,
        patch: Option<SVNum>,
        operator: SemVerOperator,
    },
    RegistryFallbackValues,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SemVerOperator {
    /// ~
    Close,
    /// ^
    Compatible,
    /// =
    Exact,
    /// >
    Greater,
    /// <
    Smaller,
}

impl SemVerOperator {
    fn as_char(&self) -> char {
        match self {
            SemVerOperator::Close => '~',
            SemVerOperator::Compatible => '^',
            SemVerOperator::Exact => '=',
            SemVerOperator::Greater => '>',
            SemVerOperator::Smaller => '<',
        }
    }
}

impl Into<String> for SemVerOperator {
    fn into(self) -> String {
        String::from(self.as_char())
    }
}

impl Into<char> for SemVerOperator {
    fn into(self) -> char {
        self.as_char()
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

const STR_NOT_A_NUMBER: &str = "Part is not a number";
const STR_MORE_THAN_2_DOTS: &str = "More than 2 dots found";

impl TryFrom<String> for SemVer {
    type Error = crate::NumngError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_string(&value)
    }
}

impl Into<String> for SemVer {
    fn into(self) -> String {
        self.to_string()
    }
}

impl SemVer {
    pub fn to_string(&self) -> String {
        match self {
            SemVer::RegistryFallbackValues => String::from("_"),
            SemVer::Custom(c) => c.clone(),
            SemVer::Latest => String::from("latest"),
            SemVer::Normal {
                major,
                minor,
                patch,
                operator,
            } => {
                let mut out = String::from(operator.as_char());
                out.push('.');
                out.push_str(major.to_string().as_str());
                if let Some(minor) = minor {
                    out.push('.');
                    out.push_str(minor.to_string().as_str());
                    if let Some(patch) = patch {
                        out.push('.');
                        out.push_str(patch.to_string().as_str());
                    }
                }
                out
            }
        }
    }

    pub fn from_string(value: &String) -> Result<Self, NumngError> {
        if value.is_empty() || value.as_str() == "latest" {
            return Ok(Self::Latest);
        }
        if value.as_str() == "_" {
            return Ok(Self::RegistryFallbackValues);
        }
        let mut text = value.clone();
        if !text.chars().into_iter().any(|c| c.is_ascii_digit()) {
            return Ok(Self::Custom(text));
        }
        let operator: SemVerOperator = if let Some(a) = text.strip_prefix("<") {
            text = String::from(a);
            SemVerOperator::Smaller
        } else if let Some(a) = text.strip_prefix(">") {
            text = String::from(a);
            SemVerOperator::Greater
        } else if let Some(a) = text.strip_prefix("~") {
            text = String::from(a);
            SemVerOperator::Close
        } else if let Some(a) = text.strip_prefix("^") {
            text = String::from(a);
            SemVerOperator::Compatible
        } else {
            SemVerOperator::Exact
        };

        let parts: Vec<SVNum> = text
            .split(".")
            .map(|i| SVNum::from_str_radix(i, 10))
            .collect::<Result<Vec<SVNum>, std::num::ParseIntError>>()
            .map_err(|_| crate::NumngError::InvalidSemVer {
                semver: value.clone(),
                issue: String::from(STR_NOT_A_NUMBER),
            })?;
        if parts.len() > 3 {
            return Err(crate::NumngError::InvalidSemVer {
                issue: String::from(STR_MORE_THAN_2_DOTS),
                semver: value.clone(),
            });
        }

        // match required since i have to deref the get
        let minor: Option<SVNum> = match parts.get(1) {
            Some(i) => Some(*i),
            None => None,
        };
        let patch: Option<SVNum> = match parts.get(2) {
            Some(i) => Some(*i),
            None => None,
        };

        Ok(Self::Normal {
            major: parts[0],
            minor,
            patch,
            operator,
        })
    }

    /// intended for checking which version within a repo is bigger.
    /// therefore it does not handle operators (except "latest")
    pub fn greater_than(&self, other: &SemVer) -> bool {
        match self {
            SemVer::RegistryFallbackValues => false,
            SemVer::Custom(_) => false,
            SemVer::Latest => true,
            SemVer::Normal {
                major,
                minor,
                patch,
                operator: _, // nope
            } => match other {
                SemVer::RegistryFallbackValues => true,
                SemVer::Custom(_) => true,
                SemVer::Latest => false,
                SemVer::Normal {
                    major: o_major,
                    minor: o_minor,
                    patch: o_patch,
                    operator: _, // nope not gonna do it
                } => {
                    major > o_major
                        || (major == o_major
                            && *minor != None
                            && (minor > o_minor
                                || (minor == o_minor
                                    && *patch != None
                                    && (*o_patch == None || patch > o_patch))))
                }
            },
        }
    }

    /// self is the pattern ("^1.2.0") and other is the actual version
    pub fn matches(&self, other: &SemVer) -> bool {
        match self {
            SemVer::RegistryFallbackValues => matches!(other, SemVer::RegistryFallbackValues),
            SemVer::Custom(c) => match other {
                SemVer::Custom(d) => c == d,
                _ => false,
            },
            SemVer::Latest => match other {
                SemVer::Custom(_) => false,
                _ => true, // anything could be latest; this has to be determined by other checks
            },
            SemVer::Normal {
                major,
                minor,
                patch,
                operator,
            } => match other {
                SemVer::RegistryFallbackValues => false, // would already have matched above
                SemVer::Latest => *operator == SemVerOperator::Greater, // nothing else allows a major version bump
                SemVer::Custom(_) => false,
                SemVer::Normal {
                    major: p_major,
                    minor: p_minor,
                    patch: p_patch,
                    operator: _, // Sorry, but no im not going to write a handler for repositories saying "this is a webserver.nu version less than 4" instead of "this is webserver.nu 3.2.1" instead of "this is webserver.nu 3.2.1"
                } => match operator {
                    SemVerOperator::Close => {
                        major == p_major
                            && (minor.unwrap_or(0) == p_minor.unwrap_or(0)
                                && (patch.unwrap_or(0) <= p_patch.unwrap_or(0)))
                    }
                    SemVerOperator::Compatible => {
                        major == p_major
                            && (minor.unwrap_or(0) < p_minor.unwrap_or(0)
                                || (minor.unwrap_or(0) == p_minor.unwrap_or(0)
                                    && (patch.unwrap_or(0) <= p_patch.unwrap_or(0))))
                    }
                    SemVerOperator::Exact => {
                        major == p_major
                            && (*minor == None
                                || (minor.unwrap() == p_minor.unwrap_or(0)
                                    && (*patch == None || patch.unwrap() == p_patch.unwrap_or(0))))
                    }
                    SemVerOperator::Greater => {
                        major < p_major
                            || (major == p_major
                                && (*minor == None
                                    || minor.unwrap() < p_minor.unwrap_or(0)
                                    || (minor.unwrap() == p_minor.unwrap_or(0)
                                        && (*patch == None
                                            || patch.unwrap() < p_patch.unwrap_or(0)))))
                    }
                    SemVerOperator::Smaller => {
                        major > p_major
                            || (major == p_major
                                && (minor.unwrap_or(0) > p_minor.unwrap_or(0)
                                    || (minor.unwrap_or(0) == p_minor.unwrap_or(0)
                                        && (patch.unwrap_or(0) > p_patch.unwrap_or(0)))))
                    }
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemVer;
    use super::SemVerOperator;

    fn fs(t: &str) -> SemVer {
        SemVer::from_string(&String::from(t)).unwrap()
    }

    #[test]
    pub fn test_from_string() {
        assert_eq!(fs(""), SemVer::Latest);
        assert_eq!(fs("latest"), SemVer::Latest);
        assert_eq!(fs("git"), SemVer::Custom(String::from("git")));
        assert_eq!(
            fs("1"),
            SemVer::Normal {
                major: 1,
                minor: None,
                patch: None,
                operator: SemVerOperator::Exact
            }
        );
        assert_eq!(
            fs("1.2"),
            SemVer::Normal {
                major: 1,
                minor: Some(2),
                patch: None,
                operator: SemVerOperator::Exact,
            }
        );
        assert_eq!(
            fs("1.2.3"),
            SemVer::Normal {
                major: 1,
                minor: Some(2),
                patch: Some(3),
                operator: SemVerOperator::Exact,
            }
        );
        assert_eq!(
            fs("~1"),
            SemVer::Normal {
                major: 1,
                minor: None,
                patch: None,
                operator: SemVerOperator::Close
            }
        );
        assert_eq!(
            fs("^1"),
            SemVer::Normal {
                major: 1,
                minor: None,
                patch: None,
                operator: SemVerOperator::Compatible
            }
        );
        assert_eq!(
            fs("<1"),
            SemVer::Normal {
                major: 1,
                minor: None,
                patch: None,
                operator: SemVerOperator::Smaller
            }
        );
        assert_eq!(
            fs(">1"),
            SemVer::Normal {
                major: 1,
                minor: None,
                patch: None,
                operator: SemVerOperator::Greater
            }
        );
        assert!(SemVer::from_string(&String::from("1.2a.3")).is_err());
        assert!(SemVer::from_string(&String::from("1.2.3a")).is_err());
        assert!(SemVer::from_string(&String::from(">>1.2.3")).is_err());
        assert!(SemVer::from_string(&String::from("1.2.3.4")).is_err());
    }

    #[test]
    fn test_is_greater() {
        assert!(!fs("1.2.3").greater_than(&fs("1.2.3")));
        assert!(fs("1.2.3").greater_than(&fs("1.2.2")));
        assert!(!fs("1.2.3").greater_than(&fs("1.2.4")));
        assert!(fs("1.2.3").greater_than(&fs("1.1.3")));
        assert!(!fs("1.2.3").greater_than(&fs("1.3.3")));
        assert!(fs("1.2.3").greater_than(&fs("0.2.3")));
        assert!(!fs("1.2.3").greater_than(&fs("2.2.3")));

        assert!(!fs("1.2").greater_than(&fs("1.2.3")));
        assert!(!fs("1.2").greater_than(&fs("1.2.2")));
        assert!(!fs("1.2").greater_than(&fs("1.2.4")));
        assert!(fs("1.2").greater_than(&fs("1.1.3")));
        assert!(!fs("1.2").greater_than(&fs("1.3.3")));
        assert!(fs("1.2").greater_than(&fs("0.2.3")));
        assert!(!fs("1.2").greater_than(&fs("2.2.3")));

        assert!(!fs("1").greater_than(&fs("1.2.3")));
        assert!(!fs("1").greater_than(&fs("1.2.2")));
        assert!(!fs("1").greater_than(&fs("1.2.4")));
        assert!(!fs("1").greater_than(&fs("1.1.3")));
        assert!(!fs("1").greater_than(&fs("1.3.3")));
        assert!(fs("1").greater_than(&fs("0.2.3")));
        assert!(!fs("1").greater_than(&fs("2.2.3")));

        assert!(fs("1.2.3").greater_than(&fs("1.2")));
        assert!(!fs("1.2.3").greater_than(&fs("1.3")));
        assert!(fs("1.2.3").greater_than(&fs("1.1")));
        assert!(fs("1.2.3").greater_than(&fs("0.2")));
        assert!(!fs("1.2.3").greater_than(&fs("2.2")));

        assert!(fs("1.2.3").greater_than(&fs("1")));
        assert!(fs("1.2.3").greater_than(&fs("0")));
        assert!(!fs("1.2.3").greater_than(&fs("2")));
    }

    #[test]
    fn test_matches() {
        assert!(fs("1.2.3").matches(&fs("1.2.3")));
        assert!(!fs("1.2.3").matches(&fs("1.2.4")));
        assert!(!fs("1.2.3").matches(&fs("1.2.2")));
        assert!(!fs("1.2.3").matches(&fs("1.3.3")));
        assert!(!fs("1.2.3").matches(&fs("1.1.3")));
        assert!(!fs("1.2.3").matches(&fs("2.2.3")));
        assert!(!fs("1.2.3").matches(&fs("0.2.3")));
        assert!(!fs("1.2.3").matches(&fs("1.2")));
        assert!(!fs("1.2.3").matches(&fs("1.3")));
        assert!(!fs("1.2.3").matches(&fs("1.1")));
        assert!(!fs("1.2.3").matches(&fs("2.2")));
        assert!(!fs("1.2.3").matches(&fs("0.2")));
        assert!(!fs("1.2.3").matches(&fs("1")));
        assert!(!fs("1.2.3").matches(&fs("2")));
        assert!(!fs("1.2.3").matches(&fs("0")));
        assert!(fs("1.2").matches(&fs("1.2.3")));
        assert!(fs("1.2").matches(&fs("1.2.4")));
        assert!(fs("1.2").matches(&fs("1.2.2")));
        assert!(!fs("1.2").matches(&fs("1.3.3")));
        assert!(!fs("1.2").matches(&fs("1.1.3")));
        assert!(!fs("1.2").matches(&fs("2.2.3")));
        assert!(!fs("1.2").matches(&fs("0.2.3")));
        assert!(fs("1").matches(&fs("1.2.3")));
        assert!(fs("1").matches(&fs("1.2.4")));
        assert!(fs("1").matches(&fs("1.2.2")));
        assert!(fs("1").matches(&fs("1.3.3")));
        assert!(fs("1").matches(&fs("1.1.3")));
        assert!(!fs("1").matches(&fs("2.2.3")));
        assert!(!fs("1").matches(&fs("0.2.3")));

        assert!(!fs("<1.2.3").matches(&fs("1.2.3")));
        assert!(!fs("<1.2.3").matches(&fs("1.2.4")));
        assert!(fs("<1.2.3").matches(&fs("1.2.2")));
        assert!(!fs("<1.2.3").matches(&fs("1.3.3")));
        assert!(fs("<1.2.3").matches(&fs("1.1.3")));
        assert!(!fs("<1.2.3").matches(&fs("2.2.3")));
        assert!(fs("<1.2.3").matches(&fs("0.2.3")));
        assert!(fs("<1.2.3").matches(&fs("1.2")));
        assert!(!fs("<1.2.3").matches(&fs("1.3")));
        assert!(fs("<1.2.3").matches(&fs("1.1")));
        assert!(!fs("<1.2.3").matches(&fs("2.2")));
        assert!(fs("<1.2.3").matches(&fs("0.2")));
        assert!(fs("<1.2.3").matches(&fs("1")));
        assert!(!fs("<1.2.3").matches(&fs("2")));
        assert!(fs("<1.2.3").matches(&fs("0")));
        assert!(!fs("<1.2").matches(&fs("1.2.3")));
        assert!(!fs("<1.2").matches(&fs("1.2.4")));
        assert!(!fs("<1.2").matches(&fs("1.2.2")));
        assert!(!fs("<1.2").matches(&fs("1.3.3")));
        assert!(fs("<1.2").matches(&fs("1.1.3")));
        assert!(!fs("<1.2").matches(&fs("2.2.3")));
        assert!(fs("<1.2").matches(&fs("0.2.3")));
        assert!(!fs("<1").matches(&fs("1.2.3")));
        assert!(!fs("<1").matches(&fs("1.2.4")));
        assert!(!fs("<1").matches(&fs("1.2.2")));
        assert!(!fs("<1").matches(&fs("1.3.3")));
        assert!(!fs("<1").matches(&fs("1.1.3")));
        assert!(!fs("<1").matches(&fs("2.2.3")));
        assert!(fs("<1").matches(&fs("0.2.3")));

        assert!(!fs(">1.2.3").matches(&fs("1.2.3")));
        assert!(fs(">1.2.3").matches(&fs("1.2.4")));
        assert!(!fs(">1.2.3").matches(&fs("1.2.2")));
        assert!(fs(">1.2.3").matches(&fs("1.3.3")));
        assert!(!fs(">1.2.3").matches(&fs("1.1.3")));
        assert!(fs(">1.2.3").matches(&fs("2.2.3")));
        assert!(!fs(">1.2.3").matches(&fs("0.2.3")));
        assert!(!fs(">1.2.3").matches(&fs("1.2")));
        assert!(fs(">1.2.3").matches(&fs("1.3")));
        assert!(!fs(">1.2.3").matches(&fs("1.1")));
        assert!(fs(">1.2.3").matches(&fs("2.2")));
        assert!(!fs(">1.2.3").matches(&fs("0.2")));
        assert!(!fs(">1.2.3").matches(&fs("1")));
        assert!(fs(">1.2.3").matches(&fs("2")));
        assert!(!fs(">1.2.3").matches(&fs("0")));
        assert!(fs(">1.2").matches(&fs("1.2.3")));
        assert!(fs(">1.2").matches(&fs("1.2.4")));
        assert!(fs(">1.2").matches(&fs("1.2.2")));
        assert!(fs(">1.2").matches(&fs("1.3.3")));
        assert!(!fs(">1.2").matches(&fs("1.1.3")));
        assert!(fs(">1.2").matches(&fs("2.2.3")));
        assert!(!fs(">1.2").matches(&fs("0.2.3")));
        assert!(fs(">1").matches(&fs("1.2.3")));
        assert!(fs(">1").matches(&fs("1.2.4")));
        assert!(fs(">1").matches(&fs("1.2.2")));
        assert!(fs(">1").matches(&fs("1.3.3")));
        assert!(fs(">1").matches(&fs("1.1.3")));
        assert!(fs(">1").matches(&fs("2.2.3")));
        assert!(!fs(">1").matches(&fs("0.2.3")));

        assert!(fs("^1.2.3").matches(&fs("1.2.3")));
        assert!(fs("^1.2.3").matches(&fs("1.2.4")));
        assert!(!fs("^1.2.3").matches(&fs("1.2.2")));
        assert!(fs("^1.2.3").matches(&fs("1.3.3")));
        assert!(!fs("^1.2.3").matches(&fs("1.1.3")));
        assert!(!fs("^1.2.3").matches(&fs("2.2.3")));
        assert!(!fs("^1.2.3").matches(&fs("0.2.3")));
        assert!(!fs("^1.2.3").matches(&fs("1.2")));
        assert!(fs("^1.2.3").matches(&fs("1.3")));
        assert!(!fs("^1.2.3").matches(&fs("1.1")));
        assert!(!fs("^1.2.3").matches(&fs("2.2")));
        assert!(!fs("^1.2.3").matches(&fs("0.2")));
        assert!(!fs("^1.2.3").matches(&fs("1")));
        assert!(!fs("^1.2.3").matches(&fs("2")));
        assert!(!fs("^1.2.3").matches(&fs("0")));
        assert!(fs("^1.2").matches(&fs("1.2.3")));
        assert!(fs("^1.2").matches(&fs("1.2.4")));
        assert!(fs("^1.2").matches(&fs("1.2.2")));
        assert!(fs("^1.2").matches(&fs("1.3.3")));
        assert!(!fs("^1.2").matches(&fs("1.1.3")));
        assert!(!fs("^1.2").matches(&fs("2.2.3")));
        assert!(!fs("^1.2").matches(&fs("0.2.3")));
        assert!(fs("^1").matches(&fs("1.2.3")));
        assert!(fs("^1").matches(&fs("1.2.4")));
        assert!(fs("^1").matches(&fs("1.2.2")));
        assert!(fs("^1").matches(&fs("1.3.3")));
        assert!(fs("^1").matches(&fs("1.1.3")));
        assert!(!fs("^1").matches(&fs("2.2.3")));
        assert!(!fs("^1").matches(&fs("0.2.3")));

        assert!(fs("~1.2.3").matches(&fs("1.2.3")));
        assert!(fs("~1.2.3").matches(&fs("1.2.4")));
        assert!(!fs("~1.2.3").matches(&fs("1.2.2")));
        assert!(!fs("~1.2.3").matches(&fs("1.3.3")));
        assert!(!fs("~1.2.3").matches(&fs("1.1.3")));
        assert!(!fs("~1.2.3").matches(&fs("2.2.3")));
        assert!(!fs("~1.2.3").matches(&fs("0.2.3")));
        assert!(!fs("~1.2.3").matches(&fs("1.2")));
        assert!(!fs("~1.2.3").matches(&fs("1.3")));
        assert!(!fs("~1.2.3").matches(&fs("1.1")));
        assert!(!fs("~1.2.3").matches(&fs("2.2")));
        assert!(!fs("~1.2.3").matches(&fs("0.2")));
        assert!(!fs("~1.2.3").matches(&fs("1")));
        assert!(!fs("~1.2.3").matches(&fs("2")));
        assert!(!fs("~1.2.3").matches(&fs("0")));
        assert!(fs("~1.2").matches(&fs("1.2.3")));
        assert!(fs("~1.2").matches(&fs("1.2.4")));
        assert!(fs("~1.2").matches(&fs("1.2.2")));
        assert!(!fs("~1.2").matches(&fs("1.3.3")));
        assert!(!fs("~1.2").matches(&fs("1.1.3")));
        assert!(!fs("~1.2").matches(&fs("2.2.3")));
        assert!(!fs("~1.2").matches(&fs("0.2.3")));
        assert!(!fs("~1").matches(&fs("1.2.3")));
        assert!(!fs("~1").matches(&fs("1.2.4")));
        assert!(!fs("~1").matches(&fs("1.2.2")));
        assert!(!fs("~1").matches(&fs("1.3.3")));
        assert!(!fs("~1").matches(&fs("1.1.3")));
        assert!(!fs("~1").matches(&fs("2.2.3")));
        assert!(!fs("~1").matches(&fs("0.2.3")));
    }
}
