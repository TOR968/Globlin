use super::*;
use crate::config::Sources;

fn only(kinds: &[SourceKind]) -> Config {
    let mut config = Config {
        sources: Sources {
            npm: false,
            bun: false,
            pnpm: false,
            yarn: false,
            winget: false,
            choco: false,
        },
        ..Default::default()
    };
    for kind in kinds {
        config.set_source_enabled(*kind, true);
    }
    config
}

#[test]
fn disabling_every_source_is_an_error_rather_than_an_empty_result() {
    let error = enabled(&only(&[]))
        .err()
        .expect("disabling every source should fail")
        .to_string();
    assert!(error.contains("no package sources are enabled"), "{error}");
}

#[test]
fn npm_alone_is_enough_to_run() {
    let sources = enabled(&only(&[SourceKind::Npm])).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].kind(), SourceKind::Npm);
}

#[test]
fn a_missing_optional_source_does_not_break_the_working_one() {
    let sources = enabled(&only(&[SourceKind::Npm, SourceKind::Bun])).unwrap();
    assert!(sources
        .iter()
        .any(|source| source.kind() == SourceKind::Npm));
}

#[test]
fn a_hidden_command_targets_the_program_it_was_given() {
    let command = hidden_command(Path::new("npm.cmd"));
    assert_eq!(command.get_program(), "npm.cmd");
}

#[test]
fn a_program_that_is_not_on_path_is_not_found() {
    assert_eq!(find_on_path("globlin-absent-4b1e.exe"), None);
}

#[test]
fn every_source_kind_round_trips_through_its_label() {
    for kind in KINDS {
        assert_eq!(SourceKind::from_label(kind.label()), Some(kind));
    }
}

#[test]
fn only_the_enabled_sources_are_built() {
    let config = only(&[SourceKind::Npm]);

    assert!(config.source_enabled(SourceKind::Npm));
    assert!(!config.source_enabled(SourceKind::Winget));
}

#[test]
fn a_node_modules_root_without_a_manifest_is_empty_not_a_panic() {
    let root = std::env::temp_dir().join("globlin-test-absent-root");

    assert!(node_modules_listing(&root, SourceKind::Yarn).is_empty());
}
