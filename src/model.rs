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

    pub fn done(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.is_some())
            .count()
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
    SelfUpdate,
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
mod tests;
