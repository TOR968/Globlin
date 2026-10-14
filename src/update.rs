use std::time::Duration;

use crate::config::Config;
use crate::diagnostics;
use crate::model::{
    Blocked, BlockedScript, Installed, Package, Refusal, RowState, SourceKind, Status, UpdateTarget,
};
use crate::source::{self, PackageSource};

const BUSY_PAUSE: Duration = Duration::from_secs(30);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub updated: Vec<UpdateTarget>,
    pub failed: Vec<String>,
    pub skipped: Vec<Skipped>,
    pub blocked: Vec<Blocked>,
    pub retried: bool,
}

impl Outcome {
    pub fn changed_packages(&self) -> bool {
        !self.updated.is_empty() || !self.failed.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub name: String,
    pub reason: Refusal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Started {
        target: UpdateTarget,
        index: usize,
        total: usize,
    },
    Finished {
        index: usize,
        state: RowState,
    },
}

struct Failure {
    details: String,
    blocked: Option<Vec<BlockedScript>>,
    refusal: Option<Refusal>,
}

impl From<String> for Failure {
    fn from(details: String) -> Self {
        Self {
            details,
            blocked: None,
            refusal: None,
        }
    }
}

pub fn run(
    config: &Config,
    targets: &[UpdateTarget],
    approvals: &[Blocked],
    announce: impl Fn(Step),
) -> Outcome {
    let sources = match source::enabled(config) {
        Ok(sources) => sources,
        Err(error) => {
            return fail_all(
                targets,
                &format!(
                    "no package source is available: {error}
"
                ),
            )
        }
    };
    if let Err(details) = approvals
        .iter()
        .try_for_each(|blocked| approve(&sources, blocked))
    {
        return fail_all(targets, &details);
    }

    let outcome = drive(
        targets,
        jobs(targets, &config.one_prompt_kinds()),
        |job| execute(&sources, targets, job),
        &announce,
        BUSY_PAUSE,
    );
    Outcome {
        retried: !approvals.is_empty(),
        ..outcome
    }
}

fn drive(
    targets: &[UpdateTarget],
    jobs: Vec<Vec<usize>>,
    execute: impl Fn(&[usize]) -> Vec<Result<(), Failure>>,
    announce: &impl Fn(Step),
    busy_pause: Duration,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut report = String::new();
    let mut busy = Vec::new();

    for job in jobs {
        announce_start(announce, targets, job[0]);
        for (index, result) in job.iter().copied().zip(execute(&job)) {
            match result {
                Err(failure) if failure.refusal == Some(Refusal::Busy) => busy.push(index),
                result => record(
                    &mut outcome,
                    &mut report,
                    &targets[index],
                    result,
                    |state| {
                        announce(Step::Finished { index, state });
                    },
                ),
            }
        }
    }

    if !busy.is_empty() {
        std::thread::sleep(busy_pause);
    }
    for index in busy {
        announce_start(announce, targets, index);
        for result in execute(&[index]) {
            record(
                &mut outcome,
                &mut report,
                &targets[index],
                result,
                |state| {
                    announce(Step::Finished { index, state });
                },
            );
        }
    }

    outcome
}

fn announce_start(announce: &impl Fn(Step), targets: &[UpdateTarget], index: usize) {
    announce(Step::Started {
        target: targets[index].clone(),
        index,
        total: targets.len(),
    });
}

fn record(
    outcome: &mut Outcome,
    report: &mut String,
    target: &UpdateTarget,
    result: Result<(), Failure>,
    finished: impl FnOnce(RowState),
) {
    let Err(failure) = result else {
        outcome.updated.push(target.clone());
        return finished(RowState::Done);
    };
    report.push_str(&failure.details);
    diagnostics::record_failures(report);
    let state = match (failure.blocked, failure.refusal) {
        (Some(scripts), _) => {
            outcome.blocked.push(Blocked {
                target: target.clone(),
                scripts,
            });
            RowState::Failed
        }
        (None, Some(reason @ (Refusal::NotApplicable | Refusal::Cancelled))) => {
            outcome.skipped.push(Skipped {
                name: target.name.clone(),
                reason,
            });
            RowState::Skipped
        }
        (None, _) => {
            outcome.failed.push(target.name.clone());
            RowState::Failed
        }
    };
    finished(state);
}

fn execute(
    sources: &[Box<dyn PackageSource>],
    targets: &[UpdateTarget],
    job: &[usize],
) -> Vec<Result<(), Failure>> {
    if let [index] = job {
        return vec![apply(sources, &targets[*index])];
    }
    let group: Vec<UpdateTarget> = job.iter().map(|index| targets[*index].clone()).collect();
    apply_together(sources, &group)
}

pub fn toast(outcome: &Outcome) -> Option<(&'static str, String)> {
    if !outcome.failed.is_empty() {
        return Some((
            "Globlin — update failed",
            format!("{} (see Open last log)", outcome.failed.join(", ")),
        ));
    }
    let lines: Vec<String> = outcome
        .updated
        .iter()
        .map(|target| format!("{}{}", target.name, target.source.suffix()))
        .chain(
            outcome
                .skipped
                .iter()
                .map(|skipped| format!("{} — {}", skipped.name, skipped.reason.reason())),
        )
        .collect();
    if lines.is_empty() {
        return None;
    }
    let title = if outcome.updated.is_empty() {
        "Globlin — nothing updated"
    } else {
        "Globlin — updated"
    };
    Some((
        title,
        lines.join(
            "
",
        ),
    ))
}

fn jobs(targets: &[UpdateTarget], one_prompt: &[SourceKind]) -> Vec<Vec<usize>> {
    let mut jobs: Vec<Vec<usize>> = Vec::new();
    let mut shared: Vec<(SourceKind, usize)> = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        if one_prompt.contains(&target.source) {
            if let Some((_, job)) = shared.iter().find(|(kind, _)| *kind == target.source) {
                jobs[*job].push(index);
                continue;
            }
            shared.push((target.source, jobs.len()));
        }
        jobs.push(vec![index]);
    }
    jobs
}

pub fn settle(approvals: &mut Vec<Blocked>, outcome: &Outcome) {
    approvals.retain(|entry| {
        !outcome
            .updated
            .iter()
            .any(|done| done.is(&entry.target.name, entry.target.source))
            && !outcome
                .blocked
                .iter()
                .any(|fresh| fresh.target.is(&entry.target.name, entry.target.source))
    });
    approvals.extend(outcome.blocked.iter().cloned());
}

pub fn mark_updated(packages: &mut [Package], outcome: &Outcome) {
    for done in &outcome.updated {
        if let Some(package) = packages
            .iter_mut()
            .find(|package| done.is(&package.name, package.source))
        {
            package.current.clone_from(&done.to);
            package.status = Status::Current;
        }
    }
}

pub fn auto_approvals(outcome: &Outcome, enabled: bool) -> Vec<Blocked> {
    if enabled && !outcome.retried {
        outcome.blocked.clone()
    } else {
        Vec::new()
    }
}

fn fail_all(targets: &[UpdateTarget], report: &str) -> Outcome {
    diagnostics::record_failures(report);
    Outcome {
        failed: targets.iter().map(|target| target.name.clone()).collect(),
        ..Outcome::default()
    }
}

fn approve(sources: &[Box<dyn PackageSource>], blocked: &Blocked) -> Result<(), String> {
    source::source_for(sources, &blocked.target.name, blocked.target.source)?
        .approve_scripts(&blocked.scripts)
        .map_err(|error| {
            format!(
                "{}: could not approve install scripts: {error}\n",
                blocked.target.name
            )
        })
}

fn apply(sources: &[Box<dyn PackageSource>], target: &UpdateTarget) -> Result<(), Failure> {
    let source = source::source_for(sources, &target.name, target.source)?;
    let output = source::run_change(
        source.update_to(&target.name, &target.to),
        &target.name,
        target.source,
    )?;

    if output.status.success() {
        return Ok(());
    }
    Err(Failure {
        blocked: source.blocked_scripts(&String::from_utf8_lossy(&output.stderr)),
        refusal: source.refusal(&output),
        details: describe_failure(target, &output),
    })
}

fn apply_together(
    sources: &[Box<dyn PackageSource>],
    group: &[UpdateTarget],
) -> Vec<Result<(), Failure>> {
    let first = &group[0];
    let names: Vec<&str> = group.iter().map(|target| target.name.as_str()).collect();
    let ran = source::source_for(sources, &first.name, first.source).and_then(|source| {
        let output = source::run_change(
            source.update_together(&names),
            &names.join(", "),
            first.source,
        )?;
        Ok((source, output))
    });
    let (source, output) = match ran {
        Ok(ran) => ran,
        Err(details) => return fail_group(group, details),
    };

    let installed = source.installed().ok();
    let mut details = Some(diagnostics::command_report(
        &format!(
            "{} via {} under one prompt",
            names.join(", "),
            first.source.label()
        ),
        &output,
    ));
    changed_versions(group, installed.as_deref(), output.status.success())
        .into_iter()
        .map(|changed| {
            if changed {
                Ok(())
            } else {
                Err(Failure::from(details.take().unwrap_or_default()))
            }
        })
        .collect()
}

fn fail_group(group: &[UpdateTarget], details: String) -> Vec<Result<(), Failure>> {
    let mut details = Some(details);
    group
        .iter()
        .map(|_| Err(Failure::from(details.take().unwrap_or_default())))
        .collect()
}

fn changed_versions(
    group: &[UpdateTarget],
    installed: Option<&[Installed]>,
    succeeded: bool,
) -> Vec<bool> {
    group
        .iter()
        .map(|target| match installed {
            Some(listing) => listing
                .iter()
                .any(|item| item.name == target.name && item.version != target.from),
            None => succeeded,
        })
        .collect()
}

fn describe_failure(target: &UpdateTarget, output: &std::process::Output) -> String {
    diagnostics::command_report(
        &format!(
            "{} {} → {} via {}",
            target.name,
            target.from,
            target.to,
            target.source.label()
        ),
        output,
    )
}

#[cfg(test)]
mod tests;
