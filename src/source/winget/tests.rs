use super::*;

const TABLE: &str = "Name                 Id                     Version      Available    Source
---------------------------------------------------------------------------
Git                  Git.Git                2.44.0       2.47.1       winget
Microsoft Edge       Microsoft.Edge         120.0.1                   winget
Node.js              OpenJS.NodeJS          22.11.0      24.1.0       winget
";

const WITHOUT_AVAILABLE: &str = "Name             Id               Version      Source
------------------------------------------------------
Git              Git.Git          2.44.0       winget
";

#[test]
fn the_id_is_the_key_not_the_display_name() {
    let installed = parse_table(TABLE);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["Git.Git", "Microsoft.Edge", "OpenJS.NodeJS"]);
}

#[test]
fn the_available_column_becomes_the_reported_upgrade() {
    let installed = parse_table(TABLE);

    assert_eq!(installed[0].version, "2.44.0");
    assert_eq!(installed[0].available.as_deref(), Some("2.47.1"));
    assert_eq!(installed[0].source, SourceKind::Winget);
}

#[test]
fn a_blank_available_cell_means_the_package_is_not_behind() {
    let installed = parse_table(TABLE);

    assert_eq!(installed[1].name, "Microsoft.Edge");
    assert_eq!(installed[1].available, None);
}

#[test]
fn a_table_without_an_available_column_reports_no_upgrades() {
    let installed = parse_table(WITHOUT_AVAILABLE);

    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "Git.Git");
    assert_eq!(installed[0].available, None);
}

#[test]
fn versions_that_are_not_semver_survive_the_parse() {
    let table = "Name    Id           Version        Available      Source
------------------------------------------------------------
Thing   Some.Thing   1.2.3.4        2024.01.15     winget
";
    let installed = parse_table(table);

    assert_eq!(installed[0].version, "1.2.3.4");
    assert_eq!(installed[0].available.as_deref(), Some("2024.01.15"));
}

#[test]
fn progress_noise_before_the_header_is_ignored() {
    let noisy = format!("  -\n  \\\n  |\n{TABLE}");

    assert_eq!(parse_table(&noisy).len(), 3);
}

#[test]
fn output_without_a_table_is_empty_not_a_panic() {
    assert!(parse_table("No installed package found matching input criteria.").is_empty());
    assert!(parse_table("").is_empty());
}

#[test]
fn a_byte_order_mark_does_not_hide_the_first_column() {
    let installed = parse_table(&format!("\u{feff}{TABLE}"));

    assert_eq!(installed[0].name, "Git.Git");
}

#[test]
#[ignore = "runs the real winget: cargo test -- --ignored --exact source::winget::tests::the_real_winget_listing_still_has_the_shape_the_parser_expects"]
fn the_real_winget_listing_still_has_the_shape_the_parser_expects() {
    let installed = Winget::new().unwrap().installed().unwrap();

    assert!(!installed.is_empty(), "winget listed nothing at all");
    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
    assert!(installed
        .iter()
        .all(|item| item.source == SourceKind::Winget));
}
