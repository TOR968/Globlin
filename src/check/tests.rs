use super::*;
use crate::model::SourceKind;

fn installed(name: &str, version: &str) -> Installed {
    Installed::new(name, version, SourceKind::Npm)
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
            latest: "2.146.3".to_string()
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
            latest: "2.10.0".to_string()
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
fn the_same_name_from_two_sources_is_looked_up_once() {
    let items = vec![
        installed("typescript", "7.0.2"),
        Installed::new("typescript", "5.9.3", SourceKind::Bun),
    ];
    assert_eq!(
        lookup_names(&items, &Config::default()),
        vec!["typescript".to_string()]
    );
}

#[test]
fn a_failed_release_lookup_leaves_the_report_without_one() {
    assert_eq!(look_up_release(Err("no network".into())), None);
}

#[test]
fn a_successful_release_lookup_is_carried_in_the_report() {
    let release = crate::selfupdate::Release {
        version: Version::parse("0.2.0").unwrap(),
        exe_url: "https://example.test/exe".to_string(),
        sha_url: "https://example.test/sha".to_string(),
    };
    assert_eq!(look_up_release(Ok(Some(release.clone()))), Some(release));
}

#[test]
fn a_failed_package_check_still_carries_the_release() {
    let release = crate::selfupdate::Release {
        version: Version::parse("0.2.0").unwrap(),
        exe_url: "https://example.test/exe".to_string(),
        sha_url: "https://example.test/sha".to_string(),
    };
    let report = Report {
        packages: Err("npm is not installed".into()),
        release: Some(release.clone()),
    };
    assert!(report.packages.is_err());
    assert_eq!(report.release, Some(release));
}

#[test]
fn a_self_reporting_source_is_never_looked_up_in_the_npm_registry() {
    let items = vec![
        installed("prettier", "3.9.6"),
        Installed::new("Git.Git", "2.44.0", SourceKind::Winget),
    ];

    assert_eq!(
        lookup_names(&items, &Config::default()),
        vec!["prettier".to_string()]
    );
}

#[test]
fn a_self_reported_upgrade_is_the_outdated_version() {
    let package = classify(
        Installed::new("Git.Git", "2.44.0", SourceKind::Winget)
            .with_available(Some("2.47.1".to_string())),
        &Config::default(),
        &HashMap::new(),
    );

    assert_eq!(
        package.status,
        Status::Outdated {
            latest: "2.47.1".to_string()
        }
    );
}

#[test]
fn a_self_reporting_source_without_an_upgrade_is_current_not_unknown() {
    let package = classify(
        Installed::new("Git.Git", "2.44.0", SourceKind::Winget),
        &Config::default(),
        &HashMap::new(),
    );

    assert_eq!(package.status, Status::Current);
}

#[test]
fn a_version_the_registry_answered_for_but_npm_reported_oddly_is_unknown() {
    let package = classify(
        Installed::new("prettier", "not-a-version", SourceKind::Npm),
        &Config::default(),
        &available(&[("prettier", "3.9.6")]),
    );

    assert_eq!(package.status, Status::Unknown);
}
