use super::*;

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

#[test]
fn the_global_dir_sits_under_the_install_root() {
    let dir = global_dir();
    assert!(dir.ends_with("install/global") || dir.ends_with("install\\global"));
}
