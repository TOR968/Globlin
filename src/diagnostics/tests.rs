use super::*;
use crate::model::SourceKind;
use semver::Version;

fn package(name: &str, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: Version::parse("1.2.3").unwrap(),
        source: SourceKind::Npm,
        status,
    }
}

#[test]
fn a_line_is_tab_separated_and_ends_with_a_newline() {
    let rendered = line(&package("prettier", Status::Current));
    assert_eq!(rendered, "npm\tprettier\t1.2.3\tcurrent\n");
    assert_eq!(rendered.split('\t').count(), 4);
}

#[test]
fn an_outdated_line_carries_the_target_version() {
    let rendered = line(&package(
        "vercel",
        Status::Outdated {
            latest: Version::parse("2.0.0").unwrap(),
        },
    ));
    assert!(rendered.contains("outdated -> 2.0.0"), "{rendered}");
}

#[test]
fn every_status_has_a_distinct_description() {
    let descriptions = [
        describe(&Status::Current),
        describe(&Status::Unknown),
        describe(&Status::Ignored),
        describe(&Status::Outdated {
            latest: Version::parse("2.0.0").unwrap(),
        }),
    ];
    let unique: std::collections::HashSet<&String> = descriptions.iter().collect();
    assert_eq!(unique.len(), descriptions.len());
}

#[test]
fn the_diagnostic_files_do_not_collide() {
    assert_ne!(SNAPSHOT, FAILURES);
    assert_ne!(SNAPSHOT, SELF_UPDATE_FAILURES);
    assert_ne!(FAILURES, SELF_UPDATE_FAILURES);
    assert!(log_path().ends_with(FAILURES));
    assert!(self_update_log_path().ends_with(SELF_UPDATE_FAILURES));
    assert_ne!(log_path(), self_update_log_path());
}

fn scratch(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("globlin-test-{label}"));
    std::fs::remove_file(&path).ok();
    path
}

#[test]
fn a_note_appended_after_a_failure_leaves_both_texts_in_the_file() {
    let path = scratch("diagnostics-note-after-failure");
    truncate(&path, "failure report\n");
    append(&path, "bun note\n");

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("failure report"), "{contents}");
    assert!(contents.contains("bun note"), "{contents}");
}

#[test]
fn a_failure_written_after_a_note_replaces_everything() {
    let path = scratch("diagnostics-failure-after-note");
    append(&path, "bun note\n");
    truncate(&path, "failure report\n");

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(!contents.contains("bun note"), "{contents}");
    assert!(contents.contains("failure report"), "{contents}");
}
