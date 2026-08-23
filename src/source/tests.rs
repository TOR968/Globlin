use super::*;
use crate::config::Sources;

fn with_sources(npm: bool, bun: bool) -> Config {
    Config {
        sources: Sources { npm, bun },
        ..Default::default()
    }
}

#[test]
fn disabling_every_source_is_an_error_rather_than_an_empty_result() {
    let error = enabled(&with_sources(false, false))
        .err()
        .expect("disabling every source should fail")
        .to_string();
    assert!(error.contains("no package sources are enabled"), "{error}");
}

#[test]
fn npm_alone_is_enough_to_run() {
    let sources = enabled(&with_sources(true, false)).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].kind(), SourceKind::Npm);
}

#[test]
fn a_missing_optional_source_does_not_break_the_working_one() {
    let sources = enabled(&with_sources(true, true)).unwrap();
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
