use crate::config::Config;
use crate::diagnostics;
use crate::model::{Blocked, BlockedScript, UpdateTarget};
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

    for (index, target) in targets.iter().enumerate() {
        announce(Step::Started {
            target: target.clone(),
            index,
            total: targets.len(),
        });

        match apply(&sources, target) {
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

    outcome
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
