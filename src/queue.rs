use std::collections::VecDeque;

use crate::model::{Activity, Blocked, RemoveTarget, RowState, SourceKind, UpdateTarget};

pub static IDLE: Queue = Queue::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    Update {
        targets: Vec<UpdateTarget>,
        approvals: Vec<Blocked>,
        closing: bool,
    },
    Remove(RemoveTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    Job(Job),
    SelfUpdate,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Update,
    Remove,
}

impl Pending {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Update => "update",
            Self::Remove => "uninstall",
        }
    }
}

#[derive(Debug, Default)]
pub struct Queue {
    jobs: VecDeque<Job>,
    self_update: bool,
    check: bool,
}

impl Queue {
    pub const fn new() -> Self {
        Self {
            jobs: VecDeque::new(),
            self_update: false,
            check: false,
        }
    }

    pub fn push_update(
        &mut self,
        targets: Vec<UpdateTarget>,
        approvals: Vec<Blocked>,
        closing: bool,
        activity: Option<&Activity>,
    ) {
        let targets: Vec<UpdateTarget> = targets
            .into_iter()
            .filter(|target| !self.updates(&target.name, target.source, activity))
            .collect();
        if targets.is_empty() {
            return;
        }
        self.jobs.push_back(Job::Update {
            targets,
            approvals,
            closing,
        });
    }

    pub fn push_remove(&mut self, target: RemoveTarget, activity: Option<&Activity>) {
        if self.removes(&target.name, target.source, activity) {
            return;
        }
        self.jobs.push_back(Job::Remove(target));
    }

    pub fn push_self_update(&mut self) {
        self.self_update = true;
    }

    pub fn push_check(&mut self) {
        self.check = true;
    }

    pub fn next(&mut self) -> Option<Next> {
        if let Some(job) = self.jobs.pop_front() {
            return Some(Next::Job(job));
        }
        if std::mem::take(&mut self.self_update) {
            return Some(Next::SelfUpdate);
        }
        std::mem::take(&mut self.check).then_some(Next::Check)
    }

    pub fn queued_updates(&self) -> impl Iterator<Item = &UpdateTarget> {
        self.jobs.iter().flat_map(|job| match job {
            Job::Update { targets, .. } => targets.as_slice(),
            Job::Remove(_) => &[],
        })
    }

    pub fn pending(
        &self,
        name: &str,
        source: SourceKind,
        activity: Option<&Activity>,
    ) -> Option<Pending> {
        if self.removes(name, source, activity) {
            Some(Pending::Remove)
        } else if self.updates(name, source, activity) {
            Some(Pending::Update)
        } else {
            None
        }
    }

    fn updates(&self, name: &str, source: SourceKind, activity: Option<&Activity>) -> bool {
        let running = match activity {
            Some(Activity::Updating { batch }) => {
                batch.targets.iter().enumerate().any(|(position, target)| {
                    target.is(name, source)
                        && matches!(
                            batch.state_of(position),
                            RowState::Active | RowState::Queued
                        )
                })
            }
            _ => false,
        };
        running || self.queued_updates().any(|target| target.is(name, source))
    }

    fn removes(&self, name: &str, source: SourceKind, activity: Option<&Activity>) -> bool {
        let running = matches!(
            activity,
            Some(Activity::Removing { target }) if target.name == name && target.source == source
        );
        running
            || self.jobs.iter().any(|job| {
                matches!(job, Job::Remove(target) if target.name == name && target.source == source)
            })
    }
}

#[cfg(test)]
mod tests;
