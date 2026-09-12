use std::collections::{BTreeSet, HashMap};

use semver::Version;

use crate::config::Config;
use crate::diagnostics;
use crate::install;
use crate::model::{Catalog, Installed, Package, Status};
use crate::registry;
use crate::selfupdate::{self, Release};
use crate::source::{self, PackageSource};
use crate::Result;

#[derive(Debug)]
pub struct Report {
    pub packages: Result<Vec<Package>>,
    pub release: Option<Release>,
}

#[derive(Debug, Default)]
struct Latest {
    npm: HashMap<String, Version>,
    pypi: HashMap<String, String>,
    nuget: HashMap<String, String>,
    psgallery: HashMap<String, String>,
    crates: HashMap<String, Version>,
}

pub fn run(config: &Config) -> Report {
    let release = if install::winget_managed() {
        None
    } else {
        look_up_release(selfupdate::latest())
    };
    Report {
        packages: collect_packages(config),
        release,
    }
}

fn collect_packages(config: &Config) -> Result<Vec<Package>> {
    let sources = source::enabled(config)?;
    let installed = collect(&sources)?;
    let latest = Latest {
        npm: registry::npm_latest(&lookup_names(&installed, config, Catalog::Npm))?,
        pypi: registry::pypi_latest(&lookup_names(&installed, config, Catalog::PyPi)),
        crates: registry::crates_latest(&lookup_names(&installed, config, Catalog::Crates)),
        nuget: registry::nuget_latest(&lookup_names(&installed, config, Catalog::NuGet)),
        psgallery: registry::psgallery_latest(&lookup_names(
            &installed,
            config,
            Catalog::PsGallery,
        )),
    };
    let packages: Vec<Package> = installed
        .into_iter()
        .map(|item| classify(item, config, &latest))
        .collect();
    diagnostics::record_snapshot(&packages);
    Ok(packages)
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

fn collect(sources: &[Box<dyn PackageSource>]) -> Result<Vec<Installed>> {
    let mut installed = Vec::new();
    for source in sources {
        installed.extend(source.installed()?);
    }
    installed.sort_by(|left, right| (&left.name, left.source).cmp(&(&right.name, right.source)));
    Ok(installed)
}

fn lookup_names(installed: &[Installed], config: &Config, catalog: Catalog) -> Vec<String> {
    installed
        .iter()
        .filter(|item| item.source.catalog() == catalog)
        .filter(|item| !config.is_ignored(&item.name))
        .map(|item| item.name.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn classify(installed: Installed, config: &Config, latest: &Latest) -> Package {
    let status = if config.is_ignored(&installed.name) {
        Status::Ignored
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
        Some(indexed) if indexed != installed.version => Status::Outdated {
            latest: indexed.to_string(),
        },
        Some(_) => Status::Current,
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
