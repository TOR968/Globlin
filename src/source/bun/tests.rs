use super::*;
use std::path::Path;

#[test]
fn reads_dependency_names_from_the_global_manifest() {
    let raw = r#"{"dependencies":{"opencode-ai":"1.17.9","@scope/tool":"^2.0.0"}}"#;
    assert_eq!(
        parse_manifest(raw).unwrap(),
        vec!["@scope/tool".to_string(), "opencode-ai".to_string()]
    );
}

#[test]
fn an_empty_global_store_yields_no_packages() {
    assert!(parse_manifest("{}").unwrap().is_empty());
    assert!(parse_manifest(r#"{"dependencies":{}}"#).unwrap().is_empty());
}

#[test]
fn a_malformed_manifest_is_an_error() {
    assert!(parse_manifest("not json").is_err());
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("globlin-test-{label}"));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_manifest(root: &Path, raw: &str) {
    std::fs::write(root.join("package.json"), raw).unwrap();
}

fn with_installed(root: &Path, name: &str, version: &str) {
    let dir = root.join("node_modules").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        format!(r#"{{"name":"{name}","version":"{version}"}}"#),
    )
    .unwrap();
}

#[test]
fn the_first_candidate_holding_a_manifest_wins() {
    let first = scratch("bun-first");
    let second = scratch("bun-second");
    with_manifest(&first, r#"{"dependencies":{"a":"1.0.0"}}"#);
    with_manifest(&second, r#"{"dependencies":{"b":"1.0.0"}}"#);

    assert_eq!(resolve_global_dir(&[first.clone(), second]), Some(first));
}

#[test]
fn a_candidate_without_a_manifest_is_skipped_not_fatal() {
    let empty = scratch("bun-empty");
    let home = scratch("bun-home");
    with_manifest(&home, r#"{"dependencies":{"a":"1.0.0"}}"#);

    assert_eq!(resolve_global_dir(&[empty, home.clone()]), Some(home));
}

#[test]
fn a_candidate_with_a_malformed_manifest_is_skipped_not_fatal() {
    let broken = scratch("bun-broken");
    let home = scratch("bun-fallback");
    with_manifest(&broken, "not json");
    with_manifest(&home, r#"{"dependencies":{"a":"1.0.0"}}"#);

    assert_eq!(resolve_global_dir(&[broken, home.clone()]), Some(home));
}

#[test]
fn no_candidate_with_a_manifest_resolves_to_nothing() {
    let first = scratch("bun-none-first");
    let second = scratch("bun-none-second");

    assert_eq!(resolve_global_dir(&[first, second]), None);
}

#[test]
fn the_bun_install_root_is_probed_before_the_dot_bun_root_and_the_home_root() {
    let paths = candidates_from(
        Some(PathBuf::from("/explicit")),
        Some(PathBuf::from("/home/user")),
    );

    assert_eq!(
        paths,
        vec![
            PathBuf::from("/explicit").join("install").join("global"),
            PathBuf::from("/home/user")
                .join(".bun")
                .join("install")
                .join("global"),
            PathBuf::from("/home/user"),
        ]
    );
}

#[test]
fn an_unset_bun_install_leaves_only_the_home_candidates() {
    let paths = candidates_from(None, Some(PathBuf::from("/home/user")));

    assert_eq!(paths.len(), 2);
    assert_eq!(paths[1], PathBuf::from("/home/user"));
}

#[test]
fn a_dependency_missing_from_node_modules_drops_out_instead_of_failing_the_source() {
    let root = scratch("bun-partial");
    with_manifest(
        &root,
        r#"{"dependencies":{"present":"1.0.0","gone":"2.0.0"}}"#,
    );
    with_installed(&root, "present", "1.4.2");

    let bun = Bun {
        command: PathBuf::from("bun"),
        global_dir: Some(root),
    };
    let installed = bun.installed().unwrap();

    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "present");
    assert_eq!(installed[0].version, Version::parse("1.4.2").unwrap());
    assert_eq!(installed[0].source, SourceKind::Bun);
}
