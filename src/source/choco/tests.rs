use super::*;

const LISTING: &str = "git|2.44.0
nodejs|22.11.0
7zip.install|23.1.0
Chocolatey v2.3.0
";

const OUTDATED: &str = "git|2.44.0|2.47.1|false
nodejs|22.11.0|24.1.0|false
";

fn arguments(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn choco(shell: Option<&str>) -> Choco {
    Choco {
        command: PathBuf::from(r"C:\ProgramData\chocolatey\bin\choco.exe"),
        shell: shell.map(PathBuf::from),
    }
}

fn script(command: &Command) -> String {
    let arguments = arguments(command);
    assert_eq!(
        arguments[..3],
        ["-NoProfile", "-NonInteractive", "-Command"]
    );
    arguments[3].clone()
}

#[test]
fn an_upgrade_asks_for_elevation_and_runs_choco_unattended() {
    let script = script(&choco(Some("powershell")).update_command("git").unwrap());

    assert!(script.contains(
        r"Start-Process -FilePath 'C:\ProgramData\chocolatey\bin\choco.exe' -ArgumentList 'upgrade', 'git', '-y', '--no-progress'"
    ), "{script}");
    assert!(script.contains("-Verb RunAs"), "{script}");
    assert!(script.contains("-Wait -PassThru"), "{script}");
}

#[test]
fn an_uninstall_asks_for_elevation_too() {
    let script = script(&choco(Some("powershell")).uninstall_command("git").unwrap());

    assert!(
        script.contains("-ArgumentList 'uninstall', 'git', '-y'"),
        "{script}"
    );
    assert!(script.contains("-Verb RunAs"), "{script}");
}

#[test]
fn a_declined_elevation_prompt_stops_the_script_instead_of_reading_as_success() {
    let script = script(&choco(Some("powershell")).update_command("git").unwrap());

    assert!(
        script.starts_with("$ErrorActionPreference = 'Stop';"),
        "{script}"
    );
}

#[test]
fn a_pending_reboot_counts_as_a_finished_change() {
    let script = script(&choco(Some("powershell")).update_command("git").unwrap());

    assert!(script.contains("-in 1641, 3010) { exit 0 }"), "{script}");
}

#[test]
fn a_name_that_could_break_out_of_the_script_builds_no_command() {
    let choco = choco(Some("powershell"));

    for name in ["git'; Remove-Item C:\\ -Recurse; '", "a b", "", "x$(y)"] {
        assert!(choco.update_command(name).is_none(), "{name}");
        assert!(choco.uninstall_command(name).is_none(), "{name}");
    }
}

#[test]
fn without_powershell_there_is_no_way_to_elevate_and_no_command() {
    assert!(choco(None).update_command("git").is_none());
}

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
