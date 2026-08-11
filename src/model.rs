use semver::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Npm,
    Bun,
}

impl SourceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Bun => "bun",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "npm" => Some(Self::Npm),
            "bun" => Some(Self::Bun),
            _ => None,
        }
    }

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Npm => "",
            Self::Bun => " (bun)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: Version,
    pub source: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Current,
    Outdated { latest: Version },
    Unknown,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub current: Version,
    pub source: SourceKind,
    pub status: Status,
}

impl Package {
    pub const fn latest(&self) -> Option<&Version> {
        match &self.status {
            Status::Outdated { latest } => Some(latest),
            _ => None,
        }
    }

    pub fn stamp(&self) -> Option<String> {
        self.latest()
            .map(|latest| format!("{}:{}@{}", self.source.label(), self.name, latest))
    }

    pub fn update_target(&self) -> Option<UpdateTarget> {
        Some(UpdateTarget {
            name: self.name.clone(),
            source: self.source,
            from: self.current.clone(),
            to: self.latest()?.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTarget {
    pub name: String,
    pub source: SourceKind,
    pub from: Version,
    pub to: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activity {
    Checking,
    Updating {
        target: UpdateTarget,
        index: usize,
        total: usize,
    },
}

pub fn outdated(packages: &[Package]) -> Vec<&Package> {
    packages
        .iter()
        .filter(|package| package.latest().is_some())
        .collect()
}

pub fn stamps(packages: &[Package]) -> Vec<String> {
    let mut stamps: Vec<String> = packages.iter().filter_map(Package::stamp).collect();
    stamps.sort();
    stamps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, current: &str, status: Status) -> Package {
        Package {
            name: name.to_string(),
            current: Version::parse(current).unwrap(),
            source: SourceKind::Npm,
            status,
        }
    }

    #[test]
    fn only_outdated_packages_are_reported() {
        let packages = vec![
            package("a", "1.0.0", Status::Current),
            package(
                "b",
                "1.0.0",
                Status::Outdated {
                    latest: Version::parse("2.0.0").unwrap(),
                },
            ),
            package("c", "1.0.0", Status::Unknown),
            package("d", "1.0.0", Status::Ignored),
        ];

        let names: Vec<&str> = outdated(&packages)
            .iter()
            .map(|package| package.name.as_str())
            .collect();

        assert_eq!(names, vec!["b"]);
    }

    #[test]
    fn only_an_outdated_package_yields_an_update_target() {
        let current = package("a", "1.0.0", Status::Current);
        assert_eq!(current.update_target(), None);

        let behind = package(
            "b",
            "1.0.0",
            Status::Outdated {
                latest: Version::parse("2.0.0").unwrap(),
            },
        );
        let target = behind.update_target().unwrap();

        assert_eq!(target.name, "b");
        assert_eq!(target.from, Version::parse("1.0.0").unwrap());
        assert_eq!(target.to, Version::parse("2.0.0").unwrap());
        assert_eq!(target.source, SourceKind::Npm);
    }

    #[test]
    fn an_ignored_or_unknown_package_is_never_an_update_target() {
        assert_eq!(package("a", "1.0.0", Status::Ignored).update_target(), None);
        assert_eq!(package("b", "1.0.0", Status::Unknown).update_target(), None);
    }

    #[test]
    fn stamps_are_sorted_and_carry_source_and_target_version() {
        let packages = vec![
            package(
                "zzz",
                "1.0.0",
                Status::Outdated {
                    latest: Version::parse("1.2.0").unwrap(),
                },
            ),
            package(
                "aaa",
                "1.0.0",
                Status::Outdated {
                    latest: Version::parse("3.0.0").unwrap(),
                },
            ),
            package("mmm", "1.0.0", Status::Current),
        ];

        assert_eq!(
            stamps(&packages),
            vec!["npm:aaa@3.0.0".to_string(), "npm:zzz@1.2.0".to_string()]
        );
    }
}
