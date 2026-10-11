use super::*;
use crate::model::SourceKind;

fn package(name: &str, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: "1.2.3".to_string(),
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
            latest: "2.0.0".to_string(),
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
            latest: "2.0.0".to_string(),
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

#[test]
fn a_test_build_never_writes_into_the_users_data_directory() {
    let scratch = std::env::temp_dir();

    assert!(log_path().starts_with(&scratch), "{}", log_path().display());
    assert!(
        self_update_log_path().starts_with(&scratch),
        "{}",
        self_update_log_path().display()
    );
}

#[test]
fn the_same_note_is_written_to_the_log_only_once() {
    let path = std::env::temp_dir().join("globlin-note-once.log");
    std::fs::remove_file(&path).ok();

    append_once(&path, "bun: no global package.json found\n");
    append_once(&path, "bun: no global package.json found\n");
    append_once(&path, "yarn: yarn 2+ removed global installs\n");

    let log = std::fs::read_to_string(&path).unwrap();

    assert_eq!(
        log.matches("bun: no global package.json found\n").count(),
        1
    );
    assert_eq!(
        log.matches("yarn: yarn 2+ removed global installs\n")
            .count(),
        1
    );
    assert_eq!(log.matches("== Globlin ").count(), 2);
}

#[test]
fn a_command_report_carries_the_action_the_status_and_both_streams() {
    let output = std::process::Output {
        status: std::process::ExitStatus::default(),
        stdout: b"some out".to_vec(),
        stderr: b"some err".to_vec(),
    };

    let report = command_report("removing @scope/tool via npm", &output);

    assert!(
        report.starts_with("removing @scope/tool via npm exited with "),
        "{report}"
    );
    assert!(report.contains("--- stdout ---\nsome out\n"), "{report}");
    assert!(report.contains("--- stderr ---\nsome err\n"), "{report}");
}

#[test]
fn credentials_a_package_manager_echoes_never_reach_the_log() {
    let report = "npm verbose config //registry.npmjs.org/:_authToken=abc123secret\n\
                  using npm_0123456789abcdefghijABCDEFGHIJ0123456789 for auth\n\
                  Authorization: Bearer xyz\n\
                  prettier 3.9.6 is fine\n";

    let cleaned = redacted(report);

    assert!(!cleaned.contains("abc123secret"), "{cleaned}");
    assert!(!cleaned.contains("npm_0123456789abcdefghij"), "{cleaned}");
    assert!(!cleaned.contains("Bearer xyz"), "{cleaned}");
    assert!(
        cleaned.contains("//registry.npmjs.org/:_authToken=<redacted>\n"),
        "{cleaned}"
    );
    assert!(cleaned.contains("using <redacted> for auth\n"), "{cleaned}");
    assert!(cleaned.ends_with("prettier 3.9.6 is fine\n"), "{cleaned}");
}

#[test]
fn a_log_entry_is_stamped_with_the_version_and_the_utc_time() {
    let moment = UNIX_EPOCH + std::time::Duration::from_secs(1_791_676_683);

    assert_eq!(
        heading(moment),
        format!(
            "== Globlin {} · 2026-10-10 23:58:03 UTC ==\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}

#[test]
fn calendar_dates_survive_leap_days_and_century_rules() {
    assert_eq!(civil_date(0), (1970, 1, 1));
    assert_eq!(civil_date(11_016), (2000, 2, 29));
    assert_eq!(civil_date(19_782), (2024, 2, 29));
    assert_eq!(civil_date(47_540), (2100, 2, 28));
    assert_eq!(civil_date(47_541), (2100, 3, 1));
}
