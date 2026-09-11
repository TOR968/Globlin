use super::*;

const LISTING: &str = r#"[
  {
    "name": "C:\\Users\\x\\AppData\\Local\\pnpm\\global\\5",
    "dependencies": {
      "typescript": { "from": "typescript", "version": "5.6.2" },
      "@salesforce/cli": { "from": "@salesforce/cli", "version": "2.145.6" },
      "broken": { "from": "broken", "version": "latest" }
    }
  }
]"#;

#[test]
fn the_global_project_array_is_flattened_into_packages() {
    let installed = parse_listing(LISTING);
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["@salesforce/cli", "typescript"]);
    assert_eq!(installed[1].version, "5.6.2");
    assert_eq!(installed[1].source, SourceKind::Pnpm);
}

#[test]
fn an_entry_whose_version_is_not_semver_is_dropped() {
    let installed = parse_listing(LISTING);

    assert!(!installed.iter().any(|item| item.name == "broken"));
}

#[test]
fn a_single_project_object_parses_like_an_array_of_one() {
    let installed = parse_listing(r#"{"dependencies":{"vite":{"version":"6.0.1"}}}"#);

    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "vite");
}

#[test]
fn a_package_listed_by_two_projects_is_reported_once() {
    let raw = r#"[
      {"dependencies":{"vite":{"version":"6.0.1"}}},
      {"dependencies":{"vite":{"version":"6.0.1"}}}
    ]"#;

    assert_eq!(parse_listing(raw).len(), 1);
}

#[test]
fn output_that_is_not_json_is_empty_rather_than_fatal() {
    assert!(parse_listing("ERR_PNPM_NO_GLOBAL_DIR").is_empty());
    assert!(parse_listing("").is_empty());
}

fn arguments(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_pnpm_update_command_installs_the_latest_globally() {
    let pnpm = Pnpm {
        command: PathBuf::from("pnpm"),
    };

    assert_eq!(
        arguments(&pnpm.update_command("vite").unwrap()),
        vec!["add", "-g", "vite@latest"]
    );
}

#[test]
fn the_pnpm_uninstall_command_removes_the_package_globally() {
    let pnpm = Pnpm {
        command: PathBuf::from("pnpm"),
    };

    assert_eq!(
        arguments(&pnpm.uninstall_command("vite").unwrap()),
        vec!["remove", "-g", "vite"]
    );
}

#[test]
#[ignore = "runs the real pnpm: cargo test -- --ignored --exact source::pnpm::tests::the_real_pnpm_listing_still_parses"]
fn the_real_pnpm_listing_still_parses() {
    let installed = Pnpm::new().unwrap().installed().unwrap();

    assert!(installed
        .iter()
        .all(|item| !item.name.is_empty() && item.available.is_none()));
}
