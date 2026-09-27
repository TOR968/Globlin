use crate::config::Config;
use crate::diagnostics;
use crate::model::{Blocked, BlockedScript, UpdateTarget};
use crate::source::{self, PackageSource};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
    pub blocked: Vec<Blocked>,
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

    let mut outcome = Outcome::default();
    let mut report = String::new();

    for (index, target) in targets.iter().enumerate() {
        announce(Step::Started {
            target: target.clone(),
            index,
            total: targets.len(),
        });

        match apply(&sources, target) {
            Ok(()) => {
                outcome.updated.push(target.name.clone());
                announce(Step::Finished { index, ok: true });
            }
            Err(failure) => {
                report.push_str(&failure.details);
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

    if !report.is_empty() {
        diagnostics::record_failures(&report);
    }
    outcome
}

pub fn settle(approvals: &mut Vec<Blocked>, outcome: &Outcome) {
    approvals.retain(|entry| {
        !outcome.updated.contains(&entry.target.name)
            && !outcome
                .blocked
                .iter()
                .any(|fresh| fresh.target.is(&entry.target.name, entry.target.source))
    });
    approvals.extend(outcome.blocked.iter().cloned());
}

fn fail_all(targets: &[UpdateTarget], report: &str) -> Outcome {
    diagnostics::record_failures(report);
    Outcome {
        failed: targets.iter().map(|target| target.name.clone()).collect(),
        ..Outcome::default()
    }
}

fn approve(sources: &[Box<dyn PackageSource>], blocked: &Blocked) -> Result<(), String> {
    find_source(sources, &blocked.target)?
        .approve_scripts(&blocked.scripts)
        .map_err(|error| {
            format!(
                "{}: could not approve install scripts: {error}\n",
                blocked.target.name
            )
        })
}

fn find_source<'a>(
    sources: &'a [Box<dyn PackageSource>],
    target: &UpdateTarget,
) -> Result<&'a dyn PackageSource, String> {
    sources
        .iter()
        .find(|source| source.kind() == target.source)
        .map(AsRef::as_ref)
        .ok_or_else(|| {
            format!(
                "{}: the {} source is not available\n",
                target.name,
                target.source.label()
            )
        })
}

fn apply(sources: &[Box<dyn PackageSource>], target: &UpdateTarget) -> Result<(), Failure> {
    let source = find_source(sources, target)?;

    let output = source
        .update_command(&target.name)
        .ok_or_else(|| {
            format!(
                "{}: the {} source is read-only\n",
                target.name,
                target.source.label()
            )
        })?
        .output()
        .map_err(|error| {
            format!(
                "{}: could not start {}: {error}\n",
                target.name,
                target.source.label()
            )
        })?;

    if output.status.success() {
        return Ok(());
    }
    Err(Failure {
        blocked: source.blocked_scripts(&String::from_utf8_lossy(&output.stderr)),
        details: describe_failure(target, &output),
    })
}

fn describe_failure(target: &UpdateTarget, output: &std::process::Output) -> String {
    format!(
        "{} {} → {} via {} exited with {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n\n",
        target.name,
        target.from,
        target.to,
        target.source.label(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[cfg(test)]
mod tests;
