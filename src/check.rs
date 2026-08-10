use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;

use semver::Version;

use crate::config::Config;
use crate::model::{Installed, Package, SourceKind, Status};
use crate::platform;
use crate::registry;
use crate::source::{Bun, Npm, PackageSource};
use crate::Result;

#[derive(Debug, Default)]
pub struct Outcome {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
}

pub fn check(config: &Config) -> Result<Vec<Package>> {
    let sources = build_sources(config)?;
    let installed = collect(&sources)?;
    let latest = registry::latest_versions(&lookup_names(&installed, config))?;
    let packages: Vec<Package> = installed
        .into_iter()
        .map(|item| classify(item, config, &latest))
        .collect();
    write_snapshot(&packages);
    Ok(packages)
}

pub fn update(config: &Config, targets: &[(String, SourceKind)]) -> Outcome {
    let mut outcome = Outcome::default();
    let sources = match build_sources(config) {
        Ok(sources) => sources,
        Err(error) => {
            outcome.failed.push(error.to_string());
            return outcome;
        }
    };

    let mut report = String::new();
    for (name, kind) in targets {
        let Some(source) = sources.iter().find(|source| source.kind() == *kind) else {
            outcome.failed.push(name.clone());
            report.push_str(&format!("{name}: no {} source available\n", kind.label()));
            continue;
        };
        match run_update(source.as_ref(), name) {
            Ok(()) => outcome.updated.push(name.clone()),
            Err(details) => {
                outcome.failed.push(name.clone());
                report.push_str(&details);
            }
        }
    }
    if !report.is_empty() {
        fs::write(log_path(), &report).ok();
    }
    outcome
}

pub fn log_path() -> PathBuf {
    platform::data_dir().join("last-run.log")
}

fn build_sources(config: &Config) -> Result<Vec<Box<dyn PackageSource>>> {
    let mut sources: Vec<Box<dyn PackageSource>> = Vec::new();
    let mut failures = Vec::new();

    if config.sources.npm {
        match Npm::new(config.npm_cmd.as_deref()) {
            Ok(source) => sources.push(Box::new(source)),
            Err(error) => failures.push(error.to_string()),
        }
    }
    if config.sources.bun {
        match Bun::new() {
            Ok(source) => sources.push(Box::new(source)),
            Err(error) => failures.push(error.to_string()),
        }
    }

    if sources.is_empty() {
        return Err(if failures.is_empty() {
            "no package sources are enabled in the config".into()
        } else {
            failures.join("; ").into()
        });
    }
    Ok(sources)
}

fn collect(sources: &[Box<dyn PackageSource>]) -> Result<Vec<Installed>> {
    let mut installed = Vec::new();
    for source in sources {
        installed.extend(source.installed()?);
    }
    installed.sort_by(|left, right| {
        (&left.name, left.source).cmp(&(&right.name, right.source))
    });
    Ok(installed)
}

fn lookup_names(installed: &[Installed], config: &Config) -> Vec<String> {
    installed
        .iter()
        .filter(|item| !config.is_ignored(&item.name))
        .map(|item| item.name.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

fn classify(
    installed: Installed,
    config: &Config,
    latest: &HashMap<String, Version>,
) -> Package {
    let status = if config.is_ignored(&installed.name) {
        Status::Ignored
    } else {
        match latest.get(&installed.name) {
            Some(available) if *available > installed.version => Status::Outdated {
                latest: available.clone(),
            },
            Some(_) => Status::Current,
            None => Status::Unknown,
        }
    };
    Package {
        name: installed.name,
        current: installed.version,
        source: installed.source,
        status,
    }
}

fn run_update(source: &dyn PackageSource, name: &str) -> std::result::Result<(), String> {
    let output = source
        .update_command(name)
        .output()
        .map_err(|error| format!("{name}: could not start {}: {error}\n", source.kind().label()))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{} update {name} exited with {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n\n",
        source.kind().label(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn write_snapshot(packages: &[Package]) {
    let body: String = packages
        .iter()
        .map(|package| {
            format!(
                "{}\t{}\t{}\t{}\n",
                package.source.label(),
                package.name,
                package.current,
                describe(&package.status)
            )
        })
        .collect();
    fs::write(platform::data_dir().join("last-check.txt"), body).ok();
}

fn describe(status: &Status) -> String {
    match status {
        Status::Current => "current".to_string(),
        Status::Outdated { latest } => format!("outdated -> {latest}"),
        Status::Unknown => "unknown".to_string(),
        Status::Ignored => "ignored".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(name: &str, version: &str) -> Installed {
        Installed {
            name: name.to_string(),
            version: Version::parse(version).unwrap(),
            source: SourceKind::Npm,
        }
    }

    fn available(pairs: &[(&str, &str)]) -> HashMap<String, Version> {
        pairs
            .iter()
            .map(|(name, version)| (name.to_string(), Version::parse(version).unwrap()))
            .collect()
    }

    #[test]
    fn a_newer_registry_version_marks_the_package_outdated() {
        let package = classify(
            installed("@salesforce/cli", "2.145.6"),
            &Config::default(),
            &available(&[("@salesforce/cli", "2.146.3")]),
        );
        assert_eq!(
            package.status,
            Status::Outdated {
                latest: Version::parse("2.146.3").unwrap()
            }
        );
    }

    #[test]
    fn versions_compare_numerically_not_as_strings() {
        let package = classify(
            installed("prettier", "2.9.0"),
            &Config::default(),
            &available(&[("prettier", "2.10.0")]),
        );
        assert_eq!(
            package.status,
            Status::Outdated {
                latest: Version::parse("2.10.0").unwrap()
            }
        );
    }

    #[test]
    fn an_installed_prerelease_ahead_of_latest_is_not_outdated() {
        let package = classify(
            installed("@salesforce/cli", "2.147.7-rc.0"),
            &Config::default(),
            &available(&[("@salesforce/cli", "2.146.3")]),
        );
        assert_eq!(package.status, Status::Current);
    }

    #[test]
    fn an_equal_version_is_current() {
        let package = classify(
            installed("prettier", "3.9.6"),
            &Config::default(),
            &available(&[("prettier", "3.9.6")]),
        );
        assert_eq!(package.status, Status::Current);
    }

    #[test]
    fn a_package_missing_from_the_registry_reply_is_unknown_not_current() {
        let package = classify(
            installed("prettier", "3.9.6"),
            &Config::default(),
            &available(&[]),
        );
        assert_eq!(package.status, Status::Unknown);
    }

    #[test]
    fn ignored_packages_are_never_flagged_even_when_behind() {
        let package = classify(
            installed("npm", "12.0.2"),
            &Config::default(),
            &available(&[("npm", "99.0.0")]),
        );
        assert_eq!(package.status, Status::Ignored);
    }

    #[test]
    fn ignored_packages_are_not_looked_up() {
        let items = vec![installed("npm", "12.0.2"), installed("prettier", "3.9.6")];
        assert_eq!(
            lookup_names(&items, &Config::default()),
            vec!["prettier".to_string()]
        );
    }

    #[test]
    #[ignore = "installs for real: cargo test -- --ignored --exact check::tests::updates_a_package_for_real"]
    fn updates_a_package_for_real() {
        let name = std::env::var("UPDATE_TARGET")
            .expect("set UPDATE_TARGET to the package to install at @latest");

        let outcome = update(&Config::default(), &[(name.clone(), SourceKind::Npm)]);

        assert!(outcome.failed.is_empty(), "failed: {:?}", outcome.failed);
        assert_eq!(outcome.updated, vec![name]);
    }

    #[test]
    #[ignore = "spawns npm for real: cargo test -- --ignored --exact check::tests::a_failed_update_is_recorded_in_the_log"]
    fn a_failed_update_is_recorded_in_the_log() {
        let name = "npm-globals-tray-no-such-package-9d3f".to_string();

        let outcome = update(&Config::default(), &[(name.clone(), SourceKind::Npm)]);

        assert_eq!(outcome.failed, vec![name.clone()]);
        assert!(outcome.updated.is_empty());
        let log = fs::read_to_string(log_path()).expect("the failure log should exist");
        assert!(log.contains(&name), "log did not mention the package: {log}");
    }

    #[test]
    fn the_same_name_from_two_sources_is_looked_up_once() {
        let items = vec![
            installed("typescript", "7.0.2"),
            Installed {
                name: "typescript".to_string(),
                version: Version::parse("5.9.3").unwrap(),
                source: SourceKind::Bun,
            },
        ];
        assert_eq!(
            lookup_names(&items, &Config::default()),
            vec!["typescript".to_string()]
        );
    }
}
