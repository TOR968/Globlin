use super::*;
use crate::model::Batch;

fn target(name: &str) -> UpdateTarget {
    UpdateTarget {
        name: name.to_string(),
        source: SourceKind::Npm,
        from: "1.0.0".to_string(),
        to: "2.0.0".to_string(),
    }
}

fn removal(name: &str) -> RemoveTarget {
    RemoveTarget {
        name: name.to_string(),
        source: SourceKind::Npm,
    }
}

fn updating(names: &[&str]) -> Activity {
    Activity::Updating {
        batch: Batch::new(names.iter().map(|name| target(name)).collect()),
    }
}

fn queued_names(queue: &Queue) -> Vec<&str> {
    queue
        .queued_updates()
        .map(|target| target.name.as_str())
        .collect()
}

#[test]
fn an_empty_queue_has_nothing_to_run_next() {
    assert_eq!(Queue::new().next(), None);
}

#[test]
fn jobs_run_in_the_order_they_were_asked_for() {
    let mut queue = Queue::new();
    queue.push_remove(removal("a"), None);
    queue.push_update(vec![target("b")], Vec::new(), false, None);

    assert_eq!(queue.next(), Some(Next::Job(Job::Remove(removal("a")))));
    assert_eq!(
        queue.next(),
        Some(Next::Job(Job::Update {
            targets: vec![target("b")],
            approvals: Vec::new(),
            closing: false,
        }))
    );
    assert_eq!(queue.next(), None);
}

#[test]
fn queued_jobs_run_before_a_self_update_and_the_recheck_runs_last() {
    let mut queue = Queue::new();
    queue.push_check();
    queue.push_self_update();
    queue.push_remove(removal("a"), None);

    assert_eq!(queue.next(), Some(Next::Job(Job::Remove(removal("a")))));
    assert_eq!(queue.next(), Some(Next::SelfUpdate));
    assert_eq!(queue.next(), Some(Next::Check));
    assert_eq!(queue.next(), None);
}

#[test]
fn a_recheck_asked_for_twice_runs_once() {
    let mut queue = Queue::new();
    queue.push_check();
    queue.push_check();

    assert_eq!(queue.next(), Some(Next::Check));
    assert_eq!(queue.next(), None);
}

#[test]
fn a_package_still_waiting_in_the_running_batch_is_not_queued_again() {
    let activity = updating(&["a", "b"]);
    let mut queue = Queue::new();
    queue.push_update(
        vec![target("a"), target("b"), target("c")],
        Vec::new(),
        false,
        Some(&activity),
    );

    assert_eq!(queued_names(&queue), ["c"]);
}

#[test]
fn a_package_the_running_batch_already_failed_can_be_queued_again() {
    let mut activity = updating(&["a", "b"]);
    if let Activity::Updating { batch } = &mut activity {
        batch.finish(0, RowState::Failed);
        batch.start(1);
    }
    let mut queue = Queue::new();
    queue.push_update(vec![target("a")], Vec::new(), false, Some(&activity));

    assert_eq!(queued_names(&queue), ["a"]);
}

#[test]
fn a_package_already_queued_for_an_update_is_not_queued_twice() {
    let mut queue = Queue::new();
    queue.push_update(vec![target("a")], Vec::new(), false, None);
    queue.push_update(vec![target("a"), target("b")], Vec::new(), false, None);

    assert_eq!(queued_names(&queue), ["a", "b"]);
}

#[test]
fn an_update_whose_every_package_is_already_scheduled_adds_no_job() {
    let mut queue = Queue::new();
    queue.push_update(vec![target("a")], Vec::new(), false, None);
    queue.push_update(vec![target("a")], Vec::new(), false, None);

    assert!(queue.next().is_some());
    assert_eq!(queue.next(), None);
}

#[test]
fn an_uninstall_already_running_or_queued_is_not_queued_again() {
    let running = Activity::Removing {
        target: removal("a"),
    };
    let mut queue = Queue::new();
    queue.push_remove(removal("a"), Some(&running));
    queue.push_remove(removal("b"), Some(&running));
    queue.push_remove(removal("b"), Some(&running));

    assert_eq!(queue.next(), Some(Next::Job(Job::Remove(removal("b")))));
    assert_eq!(queue.next(), None);
}

#[test]
fn pending_names_what_will_happen_to_a_package() {
    let activity = updating(&["a"]);
    let mut queue = Queue::new();
    queue.push_update(vec![target("b")], Vec::new(), false, Some(&activity));
    queue.push_remove(removal("c"), Some(&activity));

    let pending = |name| queue.pending(name, SourceKind::Npm, Some(&activity));
    assert_eq!(pending("a"), Some(Pending::Update));
    assert_eq!(pending("b"), Some(Pending::Update));
    assert_eq!(pending("c"), Some(Pending::Remove));
    assert_eq!(pending("d"), None);
    assert_eq!(queue.pending("a", SourceKind::Pnpm, Some(&activity)), None);
}

#[test]
fn an_uninstall_outranks_an_update_for_the_same_package() {
    let mut queue = Queue::new();
    queue.push_update(vec![target("a")], Vec::new(), false, None);
    queue.push_remove(removal("a"), None);

    assert_eq!(
        queue.pending("a", SourceKind::Npm, None),
        Some(Pending::Remove)
    );
}
