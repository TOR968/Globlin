use super::*;
use crate::model::SourceKind;

fn installed(name: &str, version: &str) -> Installed {
    Installed::new(name, version, SourceKind::Npm)
}

fn available(pairs: &[(&str, &str)]) -> Latest {
    Latest {
        npm: pairs
            .iter()
            .map(|(name, version)| (name.to_string(), Version::parse(version).unwrap()))
            .collect(),
        ..Latest::default()
    }
}

fn on_pypi(pairs: &[(&str, &str)]) -> Latest {
    Latest {
        pypi: pairs
            .iter()
            .map(|(name, version)| (name.to_string(), (*version).to_string()))
            .collect(),
        ..Latest::default()
    }
}

fn from_pipx(name: &str, version: &str) -> Installed {
    Installed::new(name, version, SourceKind::Pipx)
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
        lookup_names(&items, &Config::default(), Catalog::Npm),
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
        lookup_names(&items, &Config::default(), Catalog::Npm),
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
        lookup_names(&items, &Config::default(), Catalog::Npm),
        vec!["prettier".to_string()]
    );
}

#[test]
fn a_self_reported_upgrade_is_the_outdated_version() {
    let package = classify(
        Installed::new("Git.Git", "2.44.0", SourceKind::Winget)
            .with_available(Some("2.47.1".to_string())),
        &Config::default(),
        &Latest::default(),
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
        &Latest::default(),
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

#[test]
fn a_pipx_tool_behind_the_feed_is_outdated() {
    let package = classify(
        from_pipx("black", "24.9.0"),
        &Config::default(),
        &on_pypi(&[("black", "24.10.0")]),
    );

    assert_eq!(
        package.status,
        Status::Outdated {
            latest: "24.10.0".to_string()
        }
    );
}

#[test]
fn a_pipx_tool_at_the_newest_release_is_current() {
    let package = classify(
        from_pipx("black", "24.10.0"),
        &Config::default(),
        &on_pypi(&[("black", "24.10.0")]),
    );

    assert_eq!(package.status, Status::Current);
}

#[test]
fn a_pipx_tool_the_feed_did_not_answer_for_is_unknown_not_current() {
    let package = classify(
        from_pipx("black", "24.9.0"),
        &Config::default(),
        &on_pypi(&[]),
    );

    assert_eq!(package.status, Status::Unknown);
}

#[test]
fn a_pipx_tool_on_a_prerelease_is_unknown_rather_than_wrongly_outdated() {
    let package = classify(
        from_pipx("black", "25.1.0rc1"),
        &Config::default(),
        &on_pypi(&[("black", "24.10.0")]),
    );

    assert_eq!(package.status, Status::Unknown);
}

#[test]
fn pipx_tools_are_looked_up_on_pypi_and_never_in_the_npm_registry() {
    let items = vec![installed("prettier", "3.9.6"), from_pipx("black", "24.9.0")];

    assert_eq!(
        lookup_names(&items, &Config::default(), Catalog::Npm),
        vec!["prettier".to_string()]
    );
    assert_eq!(
        lookup_names(&items, &Config::default(), Catalog::PyPi),
        vec!["black".to_string()]
    );
}

#[test]
fn an_app_whose_local_index_is_ahead_is_outdated() {
    let package = classify(
        Installed::new("bun", "1.3.3", SourceKind::Scoop).with_available(Some("1.4.2".to_string())),
        &Config::default(),
        &Latest::default(),
    );

    assert_eq!(
        package.status,
        Status::Outdated {
            latest: "1.4.2".to_string()
        }
    );
}

#[test]
fn an_app_matching_its_local_index_is_current() {
    let package = classify(
        Installed::new("bun", "1.4.2", SourceKind::Scoop).with_available(Some("1.4.2".to_string())),
        &Config::default(),
        &Latest::default(),
    );

    assert_eq!(package.status, Status::Current);
}

#[test]
fn an_app_with_no_local_index_entry_is_unknown_not_current() {
    let package = classify(
        Installed::new("bun", "1.4.2", SourceKind::Scoop),
        &Config::default(),
        &Latest::default(),
    );

    assert_eq!(package.status, Status::Unknown);
}
