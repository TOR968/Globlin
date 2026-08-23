use crate::config::Config;
use crate::diagnostics;
use crate::model::UpdateTarget;
use crate::source::{self, PackageSource};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
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

pub fn run(config: &Config, targets: &[UpdateTarget], announce: impl Fn(Step)) -> Outcome {
    let sources = match source::enabled(config) {
        Ok(sources) => sources,
        Err(error) => {
            let report = format!("no package source is available: {error}\n");
            diagnostics::record_failures(&report);
            return Outcome {
                updated: Vec::new(),
                failed: targets.iter().map(|target| target.name.clone()).collect(),
            };
        }
    };

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
            Err(details) => {
                outcome.failed.push(target.name.clone());
                report.push_str(&details);
                announce(Step::Finished { index, ok: false });
            }
        }
    }

    if !report.is_empty() {
        diagnostics::record_failures(&report);
    }
    outcome
}

fn apply(
    sources: &[Box<dyn PackageSource>],
    target: &UpdateTarget,
) -> std::result::Result<(), String> {
    let source = sources
        .iter()
        .find(|source| source.kind() == target.source)
        .ok_or_else(|| {
            format!(
                "{}: the {} source is not available\n",
                target.name,
                target.source.label()
            )
        })?;

    let output = source
        .update_command(&target.name)
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
    Err(describe_failure(target, &output))
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
