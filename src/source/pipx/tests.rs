use super::*;

const LISTING: &str = "black 24.10.0
poetry 1.8.4
yt-dlp 2025.9.5
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn each_short_row_is_a_tool_and_its_version() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["black", "poetry", "yt-dlp"]);
    assert_eq!(installed[0].version, "24.10.0");
    assert_eq!(installed[0].source, SourceKind::Pipx);
}

#[test]
fn a_calendar_version_survives_because_versions_are_kept_as_text() {
    let installed = parse_listing(LISTING);

    assert_eq!(installed[2].version, "2025.9.5");
}

#[test]
fn pipx_reports_no_upgrade_of_its_own_so_the_feed_has_to_answer() {
    let installed = parse_listing(LISTING);

    assert!(installed.iter().all(|item| item.available.is_none()));
}

#[test]
fn a_line_that_is_not_two_columns_is_not_a_tool() {
    assert!(parse_listing("nothing has been installed with pipx\n").is_empty());
    assert!(parse_listing("").is_empty());
}

#[test]
fn the_pipx_update_command_upgrades_the_named_tool() {
    let pipx = Pipx {
        command: PathBuf::from("pipx"),
    };

    assert_eq!(
        arguments(&pipx.update_command("black").unwrap()),
        vec!["upgrade", "black"]
    );
}

#[test]
fn the_pipx_uninstall_command_removes_the_named_tool() {
    let pipx = Pipx {
        command: PathBuf::from("pipx"),
    };

    assert_eq!(
        arguments(&pipx.uninstall_command("black").unwrap()),
        vec!["uninstall", "black"]
    );
}

#[test]
#[ignore = "runs the real pipx: cargo test -- --ignored --exact source::pipx::tests::the_real_pipx_listing_still_parses"]
fn the_real_pipx_listing_still_parses() {
    let installed = Pipx::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
