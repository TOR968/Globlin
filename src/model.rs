#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Npm,
    Bun,
    Pnpm,
    Yarn,
    Winget,
    Choco,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Catalog {
    Npm,
    SelfReported,
}

pub const KINDS: [SourceKind; 6] = [
    SourceKind::Npm,
    SourceKind::Bun,
    SourceKind::Pnpm,
    SourceKind::Yarn,
    SourceKind::Winget,
    SourceKind::Choco,
];

impl SourceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Bun => "bun",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Winget => "winget",
            Self::Choco => "choco",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        KINDS.into_iter().find(|kind| kind.label() == label)
    }

    pub const fn catalog(self) -> Catalog {
        match self {
            Self::Npm | Self::Bun | Self::Pnpm | Self::Yarn => Catalog::Npm,
            Self::Winget | Self::Choco => Catalog::SelfReported,
        }
    }

    pub const fn read_only(self) -> bool {
        matches!(self.catalog(), Catalog::SelfReported)
    }

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Npm => "",
            Self::Bun => " (bun)",
            Self::Pnpm => " (pnpm)",
            Self::Yarn => " (yarn)",
            Self::Winget => " (winget)",
            Self::Choco => " (choco)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub version: String,
    pub source: SourceKind,
    pub available: Option<String>,
}

impl Installed {
    pub fn new(name: impl Into<String>, version: impl Into<String>, source: SourceKind) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            source,
            available: None,
        }
    }

    #[must_use]
    pub fn with_available(mut self, available: Option<String>) -> Self {
        self.available = available;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Current,
    Outdated { latest: String },
    Unknown,
    Ignored,
}

impl Status {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Outdated { .. } => "outdated",
            Self::Unknown => "unknown",
            Self::Ignored => "ignored",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub current: String,
    pub source: SourceKind,
    pub status: Status,
}

impl Package {
    pub fn latest(&self) -> Option<&str> {
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
        if self.source.read_only() {
            return None;
        }
        Some(UpdateTarget {
            name: self.name.clone(),
            source: self.source,
            from: self.current.clone(),
            to: self.latest()?.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTarget {
    pub name: String,
    pub source: SourceKind,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub name: String,
    pub source: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveTarget {
    pub name: String,
    pub source: SourceKind,
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
    Removing { target: RemoveTarget },
    SelfUpdate,
}

pub fn outdated(packages: &[Package]) -> Vec<&Package> {
    packages
        .iter()
        .filter(|package| package.latest().is_some())
        .collect()
}

pub fn updatable(packages: &[Package]) -> Vec<&Package> {
    packages
        .iter()
        .filter(|package| package.update_target().is_some())
        .collect()
}

pub fn stamps(packages: &[Package]) -> Vec<String> {
    let mut stamps: Vec<String> = packages.iter().filter_map(Package::stamp).collect();
    stamps.sort();
    stamps
}

#[cfg(test)]
mod tests;
