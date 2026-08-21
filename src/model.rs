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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Done,
    Failed,
    Active,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub targets: Vec<UpdateTarget>,
    pub index: usize,
    pub results: Vec<Option<bool>>,
}

impl Batch {
    pub fn new(targets: Vec<UpdateTarget>) -> Self {
        let results = vec![None; targets.len()];
        Self {
            targets,
            index: 0,
            results,
        }
    }

    pub fn current(&self) -> Option<&UpdateTarget> {
        self.targets.get(self.index)
    }

    pub fn total(&self) -> usize {
        self.targets.len()
    }

    pub fn state_of(&self, position: usize) -> RowState {
        match self.results.get(position) {
            Some(Some(true)) => RowState::Done,
            Some(Some(false)) => RowState::Failed,
            _ if position == self.index => RowState::Active,
            _ => RowState::Queued,
        }
    }

    pub fn start(&mut self, position: usize) {
        self.index = position;
    }

    pub fn finish(&mut self, position: usize, ok: bool) {
        if let Some(slot) = self.results.get_mut(position) {
            *slot = Some(ok);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activity {
    Checking,
    Updating { batch: Batch },
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

    fn batch_of(names: &[&str]) -> Batch {
        Batch::new(
            names
                .iter()
                .map(|name| UpdateTarget {
                    name: (*name).to_string(),
                    source: SourceKind::Npm,
                    from: Version::parse("1.0.0").unwrap(),
                    to: Version::parse("2.0.0").unwrap(),
                })
                .collect(),
        )
    }

    #[test]
    fn a_fresh_batch_has_recorded_nothing() {
        let batch = batch_of(&["a", "b", "c"]);

        assert_eq!(batch.total(), 3);
        assert_eq!(batch.index, 0);
        assert_eq!(batch.results, vec![None, None, None]);
    }

    #[test]
    fn a_recorded_failure_stays_failed_for_the_rest_of_the_run() {
        let mut batch = batch_of(&["a", "b"]);
        batch.start(0);
        batch.finish(0, false);
        batch.start(1);
        batch.finish(1, true);

        assert_eq!(batch.results, vec![Some(false), Some(true)]);
        assert_eq!(batch.total(), 2);
    }

    #[test]
    fn the_batch_points_at_the_target_being_worked_on() {
        let mut batch = batch_of(&["a", "b"]);
        batch.start(1);

        assert_eq!(
            batch.current().map(|target| target.name.as_str()),
            Some("b")
        );
    }

    #[test]
    fn a_queued_target_is_never_reported_as_done() {
        let mut batch = batch_of(&["a", "b", "c"]);
        batch.start(0);

        assert_eq!(batch.state_of(0), RowState::Active);
        assert_eq!(batch.state_of(1), RowState::Queued);
        assert_eq!(batch.state_of(2), RowState::Queued);
    }

    #[test]
    fn a_finished_target_reports_its_own_outcome() {
        let mut batch = batch_of(&["a", "b"]);
        batch.start(0);
        batch.finish(0, false);
        batch.start(1);
        batch.finish(1, true);

        assert_eq!(batch.state_of(0), RowState::Failed);
        assert_eq!(batch.state_of(1), RowState::Done);
    }
}
