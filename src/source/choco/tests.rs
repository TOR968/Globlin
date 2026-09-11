use super::*;

const LISTING: &str = "git|2.44.0
nodejs|22.11.0
7zip.install|23.1.0
Chocolatey v2.3.0
";

const OUTDATED: &str = "git|2.44.0|2.47.1|false
nodejs|22.11.0|24.1.0|false
";

#[test]
fn the_local_listing_becomes_the_installed_set() {
    let installed = merge(LISTING, "");
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["git", "nodejs", "7zip.install"]);
    assert_eq!(installed[0].source, SourceKind::Choco);
}

#[test]
fn the_outdated_report_supplies_the_upgrade() {
    let installed = merge(LISTING, OUTDATED);

    assert_eq!(installed[0].version, "2.44.0");
    assert_eq!(installed[0].available.as_deref(), Some("2.47.1"));
}

#[test]
fn a_package_absent_from_the_outdated_report_is_not_behind() {
    let installed = merge(LISTING, OUTDATED);
    let seven_zip = installed.iter().find(|item| item.name == "7zip.install");

    assert_eq!(seven_zip.unwrap().available, None);
}

#[test]
fn an_outdated_row_naming_the_same_version_twice_is_not_an_upgrade() {
    let upgrades = parse_outdated("git|2.44.0|2.44.0|false\n");

    assert!(upgrades.is_empty());
}

#[test]
fn the_trailing_version_banner_is_not_a_package() {
    let installed = parse_listing(LISTING);

    assert!(!installed
        .iter()
        .any(|item| item.name.contains("Chocolatey")));
}

#[test]
fn chocolatey_one_needs_the_local_only_flag() {
    assert_eq!(major_version("1.4.0\n"), Some(1));
    assert_eq!(major_version("2.3.0\n"), Some(2));
    assert_eq!(major_version(""), None);
}

#[test]
fn empty_output_is_no_packages_rather_than_a_panic() {
    assert!(merge("", "").is_empty());
}

#[test]
#[ignore = "runs the real choco: cargo test -- --ignored --exact source::choco::tests::the_real_choco_listing_still_has_the_shape_the_parser_expects"]
fn the_real_choco_listing_still_has_the_shape_the_parser_expects() {
    let installed = Choco::new().unwrap().installed().unwrap();

    assert!(!installed.is_empty(), "choco listed nothing at all");
    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
    assert!(
        installed.len() < 500,
        "a listing this large means the remote feed was queried, not the local one"
    );
}
