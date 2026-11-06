use std::collections::{BTreeSet, HashMap};

use semver::Version;

use crate::config::Config;
use crate::diagnostics;
use crate::install;
use crate::model::{Catalog, Installed, Package, SourceKind, Status};
use crate::registry;
use crate::selfupdate::{self, Release};
use crate::source::{self, PackageSource};
use crate::Result;

#[derive(Debug)]
pub struct Report {
    pub packages: Result<Vec<Package>>,
    pub skipped: Vec<String>,
    pub refreshed: Vec<String>,
    pub release: Option<Release>,
}

#[derive(Debug, Default)]
struct Listing {
    installed: Vec<Installed>,
    skipped: Vec<String>,
}

#[derive(Debug, Default)]
struct Latest {
    npm: HashMap<String, Version>,
    pypi: HashMap<String, String>,
    nuget: HashMap<String, String>,
    psgallery: HashMap<String, String>,
    crates: HashMap<String, Version>,
}

pub fn run(config: &Config, announce: impl Fn(Option<SourceKind>)) -> Report {
    let release = if install::managed_by().is_some() {
        None
    } else {
        look_up_release(selfupdate::latest())
    };
    let (packages, skipped, refreshed) = match collect_packages(config, &announce) {
        Ok((packages, skipped, refreshed)) => (Ok(packages), skipped, refreshed),
        Err(error) => (Err(error), Vec::new(), Vec::new()),
    };
    Report {
        packages,
        skipped,
        refreshed,
        release,
    }
}

fn collect_packages(
    config: &Config,
    announce: &impl Fn(Option<SourceKind>),
) -> Result<(Vec<Package>, Vec<String>, Vec<String>)> {
    let sources = source::enabled(config)?;
    let Listing {
        installed,
        mut skipped,
    } = collect(&sources, announce)?;
    announce(None);
    let refreshed = refreshed(&installed);
    let names = |catalog| lookup_names(&installed, config, catalog);
    let latest = Latest {
        npm: reachable(registry::npm_latest(&names(Catalog::Npm)), &mut skipped),
        pypi: reachable(registry::pypi_latest(&names(Catalog::PyPi)), &mut skipped),
        crates: reachable(
            registry::crates_latest(&names(Catalog::Crates)),
            &mut skipped,
        ),
        nuget: reachable(registry::nuget_latest(&names(Catalog::NuGet)), &mut skipped),
        psgallery: reachable(
            registry::psgallery_latest(&names(Catalog::PsGallery)),
            &mut skipped,
        ),
    };
    let packages: Vec<Package> = installed
        .into_iter()
        .map(|item| classify(item, config, &latest))
        .collect();
    diagnostics::record_snapshot(&packages);
    Ok((packages, skipped, refreshed))
}

fn refreshed(installed: &[Installed]) -> Vec<String> {
    installed
        .iter()
        .filter_map(|item| {
            let previous = item.previous.as_ref()?;
            Some(format!("{} {previous} → {}", item.name, item.version))
        })
        .collect()
}

fn reachable<T>(
    lookup: Result<HashMap<String, T>>,
    skipped: &mut Vec<String>,
) -> HashMap<String, T> {
    lookup.unwrap_or_else(|error| {
        skipped.push(error.to_string());
        HashMap::new()
    })
}

fn look_up_release(outcome: Result<Option<Release>>) -> Option<Release> {
    match outcome {
        Ok(release) => release,
        Err(error) => {
            diagnostics::record_self_update_failure(&format!(
                "self-update lookup failed: {error}\n"
            ));
            None
        }
    }
}

fn collect(
    sources: &[Box<dyn PackageSource>],
    announce: &impl Fn(Option<SourceKind>),
) -> Result<Listing> {
    let mut listing = Listing::default();
    for source in sources {
        announce(Some(source.kind()));
        match source.installed() {
            Ok(found) => listing.installed.extend(found),
            Err(error) => listing
                .skipped
                .push(format!("{}: {error}", source.kind().label())),
        }
    }
    if listing.installed.is_empty() && !listing.skipped.is_empty() {
        return Err(listing.skipped.join("\n").into());
    }
    listing
        .installed
        .sort_by(|left, right| (&left.name, left.source).cmp(&(&right.name, right.source)));
    Ok(listing)
}

pub fn ignore_unversioned(packages: &mut [Package], config: &mut Config) -> bool {
    let mut changed = false;
    for package in packages
        .iter_mut()
        .filter(|package| package.source == SourceKind::Winget && package.status == Status::Unknown)
    {
        if config.auto_ignored.contains(&package.name) {
            continue;
        }
        config.auto_ignored.push(package.name.clone());
        config.set_ignored(&package.name, true);
        package.status = Status::Ignored;
        changed = true;
    }
    changed
}

fn lookup_names(installed: &[Installed], config: &Config, catalog: Catalog) -> Vec<String> {
    installed
        .iter()
        .filter(|item| item.source.catalog() == catalog)
        .filter(|item| !item.external)
        .filter(|item| !config.is_ignored(&item.name))
        .map(|item| item.name.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn classify(installed: Installed, config: &Config, latest: &Latest) -> Package {
    let status = if config.is_ignored(&installed.name) {
        Status::Ignored
    } else if installed.external {
        Status::Unknown
    } else {
        match installed.source.catalog() {
            Catalog::SelfReported => self_reported(installed.available.as_deref()),
            Catalog::Npm => from_registry(&installed, &latest.npm),
            Catalog::Crates => from_registry(&installed, &latest.crates),
            Catalog::PyPi => from_versions(&installed, &latest.pypi),
            Catalog::NuGet => from_versions(&installed, &latest.nuget),
            Catalog::PsGallery => from_versions(&installed, &latest.psgallery),
            Catalog::SelfResolved => from_resolved(&installed),
        }
    };
    Package {
        name: installed.name,
        current: installed.version,
        source: installed.source,
        status,
    }
}

fn self_reported(available: Option<&str>) -> Status {
    available.map_or(Status::Current, |latest| Status::Outdated {
        latest: latest.to_string(),
    })
}

fn from_registry(installed: &Installed, latest: &HashMap<String, Version>) -> Status {
    let Some(available) = latest.get(&installed.name) else {
        return Status::Unknown;
    };
    let Ok(current) = Version::parse(&installed.version) else {
        return Status::Unknown;
    };
    if *available > current {
        Status::Outdated {
            latest: available.to_string(),
        }
    } else {
        Status::Current
    }
}

fn from_resolved(installed: &Installed) -> Status {
    match installed.available.as_deref() {
        None => Status::Unknown,
        Some(indexed) if indexed == installed.version => Status::Current,
        Some(indexed) => match registry::is_newer(indexed, &installed.version) {
            Some(false) => Status::Current,
            Some(true) | None => Status::Outdated {
                latest: indexed.to_string(),
            },
        },
    }
}

fn from_versions(installed: &Installed, latest: &HashMap<String, String>) -> Status {
    let Some(available) = latest.get(&installed.name) else {
        return Status::Unknown;
    };
    match registry::numeric_is_newer(available, &installed.version) {
        Some(true) => Status::Outdated {
            latest: available.clone(),
        },
        Some(false) => Status::Current,
        None => Status::Unknown,
    }
}

#[cfg(test)]
mod tests;
