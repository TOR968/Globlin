use super::*;

const LISTING: &str = "ruff v0.16.5
- ruff
posting v2.6.1
- posting
harlequin v2.1.2 [required-python: >=3.10]
- harlequin
";

const OUTDATED: &str = "ruff v0.16.5 [latest: 0.16.7]
- ruff
harlequin v2.1.2 [required-python: >=3.10] [latest: 2.2.0]
- harlequin
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_tool_row_is_a_name_and_a_v_prefixed_version() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["ruff", "posting", "harlequin"]);
    assert_eq!(installed[0].version, "0.16.5");
    assert_eq!(installed[0].source, SourceKind::Uv);
}

#[test]
fn the_entrypoint_lines_under_a_tool_are_not_tools() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|item| item.name.starts_with('-')));
}

#[test]
fn an_entrypoint_whose_name_starts_with_v_is_still_not_a_tool() {
    assert!(parse_listing("vim-tool v1.0.0\n- vim\n")
        .iter()
        .all(|item| item.name == "vim-tool"));
}

#[test]
fn a_tool_missing_from_the_outdated_report_is_still_listed_because_that_report_omits_current_tools()
{
    let installed = merge(LISTING, Some(OUTDATED));
    let posting = installed
        .iter()
        .find(|item| item.name == "posting")
        .unwrap();

    assert_eq!(installed.len(), 3);
    assert_eq!(posting.available.as_deref(), Some("2.6.1"));
}

#[test]
fn uv_reports_the_upgrade_itself_so_pypi_is_never_asked() {
    let installed = merge(LISTING, Some(OUTDATED));

    assert_eq!(installed[0].available.as_deref(), Some("0.16.7"));
}

#[test]
fn a_latest_marker_behind_other_annotations_is_still_found() {
    let installed = merge(LISTING, Some(OUTDATED));

    assert_eq!(installed[2].version, "2.1.2");
    assert_eq!(installed[2].available.as_deref(), Some("2.2.0"));
}

#[test]
fn a_failed_outdated_report_leaves_every_tool_unresolved_rather_than_current() {
    assert!(merge(LISTING, None)
        .iter()
        .all(|item| item.available.is_none()));
}

#[test]
fn the_no_tools_notice_is_not_a_tool() {
    assert!(parse_listing("No tools installed\n").is_empty());
    assert!(parse_listing("").is_empty());
}

#[test]
fn the_uv_update_command_upgrades_the_named_tool() {
    let uv = Uv {
        command: PathBuf::from("uv"),
    };

    assert_eq!(
        arguments(&uv.update_command("ruff").unwrap()),
        vec!["tool", "upgrade", "ruff"]
    );
}

#[test]
fn the_uv_uninstall_command_removes_the_named_tool() {
    let uv = Uv {
        command: PathBuf::from("uv"),
    };

    assert_eq!(
        arguments(&uv.uninstall_command("ruff").unwrap()),
        vec!["tool", "uninstall", "ruff"]
    );
}

#[test]
#[ignore = "runs the real uv: cargo test -- --ignored --exact source::uv::tests::the_real_uv_listing_still_parses"]
fn the_real_uv_listing_still_parses() {
    let installed = Uv::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
