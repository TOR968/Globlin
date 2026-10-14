use crate::config::Config;
use crate::diagnostics;
use crate::model::{Blocked, BlockedScript, Installed, Package, SourceKind, Status, UpdateTarget};
use crate::source::{self, PackageSource};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub updated: Vec<UpdateTarget>,
    pub failed: Vec<String>,
    pub blocked: Vec<Blocked>,
    pub retried: bool,
}

impl Outcome {
    pub fn changed_packages(&self) -> bool {
        !self.updated.is_empty() || !self.failed.is_empty()
    }
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
        ok: bool,
    },
}

struct Failure {
    details: String,
    blocked: Option<Vec<BlockedScript>>,
}

impl From<String> for Failure {
    fn from(details: String) -> Self {
        Self {
            details,
            blocked: None,
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
                &format!("no package source is available: {error}\n"),
            )
        }
    };
    if let Err(details) = approvals
        .iter()
        .try_for_each(|blocked| approve(&sources, blocked))
    {
        return fail_all(targets, &details);
    }

    let mut outcome = Outcome {
        retried: !approvals.is_empty(),
        ..Outcome::default()
    };
    let mut report = String::new();

    for job in jobs(targets, config.choco_one_prompt) {
        announce(Step::Started {
            target: targets[job[0]].clone(),
            index: job[0],
            total: targets.len(),
        });

        let results = if let [index] = job[..] {
            vec![apply(&sources, &targets[index])]
        } else {
            let group: Vec<UpdateTarget> =
                job.iter().map(|index| targets[*index].clone()).collect();
            apply_together(&sources, &group)
        };

        for (index, result) in job.into_iter().zip(results) {
            let target = &targets[index];
            match result {
                Ok(()) => {
                    outcome.updated.push(target.clone());
                    announce(Step::Finished { index, ok: true });
                }
                Err(failure) => {
                    report.push_str(&failure.details);
                    diagnostics::record_failures(&report);
                    match failure.blocked {
                        Some(scripts) => outcome.blocked.push(Blocked {
                            target: target.clone(),
                            scripts,
                        }),
                        None => outcome.failed.push(target.name.clone()),
                    }
                    announce(Step::Finished { index, ok: false });
                }
            }
        }
    }

    outcome
}

fn jobs(targets: &[UpdateTarget], choco_one_prompt: bool) -> Vec<Vec<usize>> {
    let mut jobs: Vec<Vec<usize>> = Vec::new();
    let mut choco_job: Option<usize> = None;
    for (index, target) in targets.iter().enumerate() {
        if choco_one_prompt && target.source == SourceKind::Choco {
            if let Some(job) = choco_job {
                jobs[job].push(index);
                continue;
            }
            choco_job = Some(jobs.len());
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
