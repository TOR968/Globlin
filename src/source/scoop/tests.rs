use super::*;

const MANIFEST: &str = r#"{
    "version": "1.4.2",
    "description": "Incredibly fast JavaScript runtime, bundler, transpiler and package manager.",
    "homepage": "https://bun.sh/",
    "license": "MIT"
}"#;

const INSTALL: &str = r#"{
    "bucket": "main",
    "architecture": "64bit"
}"#;

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_installed_version_is_the_one_in_the_apps_manifest() {
    assert_eq!(version_of(MANIFEST).as_deref(), Some("1.4.2"));
}

#[test]
fn the_bucket_that_supplied_an_app_is_read_from_its_install_record() {
    assert_eq!(bucket_of(INSTALL).as_deref(), Some("main"));
}

#[test]
fn an_app_installed_from_a_url_has_no_bucket_to_compare_against() {
    assert_eq!(bucket_of(r#"{"architecture":"64bit"}"#), None);
    assert_eq!(bucket_of(r#"{"bucket":""}"#), None);
}

#[test]
fn a_manifest_that_is_not_json_is_not_a_version() {
    assert_eq!(version_of("<html>404</html>"), None);
    assert_eq!(version_of(r#"{"description":"no version here"}"#), None);
}

#[test]
fn a_root_without_an_apps_directory_is_empty_rather_than_a_panic() {
    let root = std::env::temp_dir().join("globlin-test-absent-scoop");

    assert!(listing(&root).is_empty());
}

#[test]
fn the_scoop_update_command_updates_the_named_app() {
    let scoop = Scoop {
        command: PathBuf::from("scoop"),
        root: PathBuf::from("scoop-root"),
    };

    assert_eq!(
        arguments(&scoop.update_command("bun").unwrap()),
        vec!["update", "bun"]
    );
}

#[test]
fn the_scoop_uninstall_command_removes_the_named_app() {
    let scoop = Scoop {
        command: PathBuf::from("scoop"),
        root: PathBuf::from("scoop-root"),
    };

    assert_eq!(
        arguments(&scoop.uninstall_command("bun").unwrap()),
        vec!["uninstall", "bun"]
    );
}

#[test]
#[ignore = "reads the real scoop tree: cargo test -- --ignored --exact source::scoop::tests::the_real_scoop_tree_still_parses"]
fn the_real_scoop_tree_still_parses() {
    let installed = Scoop::new().unwrap().installed().unwrap();

    assert!(
        !installed.is_empty(),
        "the apps directory was read through its current junction"
    );
    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && !item.version.is_empty()));
}
