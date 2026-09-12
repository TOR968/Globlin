use super::*;

const LISTING: &str = "cargo-edit v0.13.6:
    cargo-add
    cargo-rm
ripgrep v14.1.0:
    rg
tool-from-git v0.1.0 (https://github.com/example/tool#01234567):
    tool
";

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn each_header_line_is_a_crate_and_its_version() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["cargo-edit", "ripgrep"]);
    assert_eq!(installed[1].version, "14.1.0");
    assert_eq!(installed[1].source, SourceKind::Cargo);
}

#[test]
fn the_indented_binaries_under_a_crate_are_not_crates() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|item| item.name == "rg" || item.name == "cargo-add"));
}

#[test]
fn a_crate_installed_from_git_is_dropped_because_crates_io_cannot_answer_for_it() {
    assert!(!parse_listing(LISTING)
        .iter()
        .any(|item| item.name == "tool-from-git"));
}

#[test]
fn a_version_that_is_not_semver_is_not_a_crate() {
    assert!(parse_listing("odd vnot-a-version:\n").is_empty());
    assert!(parse_listing("").is_empty());
}

#[test]
fn the_cargo_update_command_installs_the_crate_again() {
    let cargo = Cargo {
        command: PathBuf::from("cargo"),
    };

    assert_eq!(
        arguments(&cargo.update_command("ripgrep").unwrap()),
        vec!["install", "ripgrep"]
    );
}

#[test]
fn the_cargo_uninstall_command_removes_the_crate() {
    let cargo = Cargo {
        command: PathBuf::from("cargo"),
    };

    assert_eq!(
        arguments(&cargo.uninstall_command("ripgrep").unwrap()),
        vec!["uninstall", "ripgrep"]
    );
}

#[test]
#[ignore = "runs the real cargo: cargo test -- --ignored --exact source::cargo::tests::the_real_cargo_listing_still_parses"]
fn the_real_cargo_listing_still_parses() {
    let installed = Cargo::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && semver::Version::parse(&item.version).is_ok()));
}
