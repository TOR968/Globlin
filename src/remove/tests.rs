use super::*;
use crate::config::Sources;
use crate::model::SourceKind;

fn target(name: &str) -> RemoveTarget {
    RemoveTarget {
        name: name.to_string(),
        source: SourceKind::Npm,
    }
}

#[test]
fn the_failure_report_names_the_package_the_source_and_the_status() {
    let report = describe_failure(
        &target("@scope/tool"),
        "exit code: 1",
        b"some out",
        b"some err",
    );

    assert!(report.contains("@scope/tool"), "{report}");
    assert!(report.contains("npm"), "{report}");
    assert!(report.contains("exit code: 1"), "{report}");
    assert!(report.contains("some out"), "{report}");
    assert!(report.contains("some err"), "{report}");
}

#[test]
fn removing_without_any_enabled_source_fails_instead_of_reporting_success() {
    let config = Config {
        sources: Sources {
            npm: false,
            bun: false,
        },
        ..Default::default()
    };

    assert!(!run(&config, &target("prettier")));
}

#[test]
fn a_source_that_cannot_be_started_fails_instead_of_reporting_success() {
    let path = std::env::temp_dir().join("globlin-test-remove-fake-npm");
    std::fs::write(&path, b"not an executable").unwrap();
    let config = Config {
        npm_cmd: Some(path.clone()),
        sources: Sources {
            npm: true,
            bun: false,
        },
        ..Default::default()
    };

    let removed = run(&config, &target("prettier"));
    std::fs::remove_file(&path).ok();

    assert!(!removed);
}
