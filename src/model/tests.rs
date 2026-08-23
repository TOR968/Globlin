use super::*;

fn package(name: &str, current: &str, status: Status) -> Package {
    Package {
        name: name.to_string(),
        current: Version::parse(current).unwrap(),
        source: SourceKind::Npm,
        status,
    }
}

#[test]
fn only_outdated_packages_are_reported() {
    let packages = vec![
        package("a", "1.0.0", Status::Current),
        package(
            "b",
            "1.0.0",
            Status::Outdated {
                latest: Version::parse("2.0.0").unwrap(),
            },
        ),
        package("c", "1.0.0", Status::Unknown),
        package("d", "1.0.0", Status::Ignored),
    ];

    let names: Vec<&str> = outdated(&packages)
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    assert_eq!(names, vec!["b"]);
}

#[test]
fn only_an_outdated_package_yields_an_update_target() {
    let current = package("a", "1.0.0", Status::Current);
    assert_eq!(current.update_target(), None);

    let behind = package(
        "b",
        "1.0.0",
        Status::Outdated {
            latest: Version::parse("2.0.0").unwrap(),
        },
    );
    let target = behind.update_target().unwrap();

    assert_eq!(target.name, "b");
    assert_eq!(target.from, Version::parse("1.0.0").unwrap());
    assert_eq!(target.to, Version::parse("2.0.0").unwrap());
    assert_eq!(target.source, SourceKind::Npm);
}

#[test]
fn an_ignored_or_unknown_package_is_never_an_update_target() {
    assert_eq!(package("a", "1.0.0", Status::Ignored).update_target(), None);
    assert_eq!(package("b", "1.0.0", Status::Unknown).update_target(), None);
}

#[test]
fn stamps_are_sorted_and_carry_source_and_target_version() {
    let packages = vec![
        package(
            "zzz",
            "1.0.0",
            Status::Outdated {
                latest: Version::parse("1.2.0").unwrap(),
            },
        ),
        package(
            "aaa",
            "1.0.0",
            Status::Outdated {
                latest: Version::parse("3.0.0").unwrap(),
            },
        ),
        package("mmm", "1.0.0", Status::Current),
    ];

    assert_eq!(
        stamps(&packages),
        vec!["npm:aaa@3.0.0".to_string(), "npm:zzz@1.2.0".to_string()]
    );
}

fn batch_of(names: &[&str]) -> Batch {
    Batch::new(
        names
            .iter()
            .map(|name| UpdateTarget {
                name: (*name).to_string(),
                source: SourceKind::Npm,
                from: Version::parse("1.0.0").unwrap(),
                to: Version::parse("2.0.0").unwrap(),
            })
            .collect(),
    )
}

#[test]
fn a_fresh_batch_has_recorded_nothing() {
    let batch = batch_of(&["a", "b", "c"]);

    assert_eq!(batch.total(), 3);
    assert_eq!(batch.index, 0);
    assert_eq!(batch.results, vec![None, None, None]);
}

#[test]
fn a_recorded_failure_stays_failed_for_the_rest_of_the_run() {
    let mut batch = batch_of(&["a", "b"]);
    batch.start(0);
    batch.finish(0, false);
    batch.start(1);
    batch.finish(1, true);

    assert_eq!(batch.results, vec![Some(false), Some(true)]);
    assert_eq!(batch.total(), 2);
}

#[test]
fn the_batch_points_at_the_target_being_worked_on() {
    let mut batch = batch_of(&["a", "b"]);
    batch.start(1);

    assert_eq!(
        batch.current().map(|target| target.name.as_str()),
        Some("b")
    );
}

#[test]
fn a_queued_target_is_never_reported_as_done() {
    let mut batch = batch_of(&["a", "b", "c"]);
    batch.start(0);

    assert_eq!(batch.state_of(0), RowState::Active);
    assert_eq!(batch.state_of(1), RowState::Queued);
    assert_eq!(batch.state_of(2), RowState::Queued);
}

#[test]
fn a_finished_target_reports_its_own_outcome() {
    let mut batch = batch_of(&["a", "b"]);
    batch.start(0);
    batch.finish(0, false);
    batch.start(1);
    batch.finish(1, true);

    assert_eq!(batch.state_of(0), RowState::Failed);
    assert_eq!(batch.state_of(1), RowState::Done);
}

#[test]
fn the_batch_counts_only_the_targets_it_has_finished() {
    let mut batch = batch_of(&["a", "b", "c"]);
    assert_eq!(batch.done(), 0);

    batch.finish(0, true);
    batch.finish(1, false);

    assert_eq!(batch.done(), 2);
}

#[test]
fn a_self_update_is_an_activity_like_any_other() {
    assert_ne!(Activity::SelfUpdate, Activity::Checking);
}
