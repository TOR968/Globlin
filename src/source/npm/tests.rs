use super::*;

const LISTING: &[u8] = br#"{
  "name": "npm",
  "problems": ["invalid: @kilocode/cli@ C:\\Users\\x\\AppData\\Roaming\\npm\\node_modules\\@kilocode"],
  "dependencies": {
    "prettier": { "version": "3.9.6", "resolved": "https://registry.npmjs.org/prettier/-/prettier-3.9.6.tgz" },
    "@salesforce/cli": { "version": "2.145.6" },
    "vanished": { "required": "^1.0.0", "missing": true },
    "not-semver": { "version": "latest" }
  }
}"#;

#[test]
fn parses_scoped_and_plain_packages() {
    let installed = parse_listing(LISTING).unwrap();
    let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

    assert_eq!(names, vec!["@salesforce/cli", "prettier"]);
}

#[test]
fn skips_entries_without_a_usable_version() {
    let installed = parse_listing(LISTING).unwrap();

    assert!(!installed.iter().any(|item| item.name == "vanished"));
    assert!(!installed.iter().any(|item| item.name == "not-semver"));
}

#[test]
fn reads_the_version_and_tags_the_source() {
    let installed = parse_listing(LISTING).unwrap();
    let prettier = installed
        .iter()
        .find(|item| item.name == "prettier")
        .unwrap();

    assert_eq!(prettier.version, Version::parse("3.9.6").unwrap());
    assert_eq!(prettier.source, SourceKind::Npm);
}

#[test]
fn a_listing_without_dependencies_is_empty_not_an_error() {
    let installed = parse_listing(br#"{"name":"npm"}"#).unwrap();
    assert!(installed.is_empty());
}

#[test]
fn unparseable_output_is_an_error() {
    assert!(parse_listing(b"").is_err());
    assert!(parse_listing(b"npm ERR! code ENOENT").is_err());
}
