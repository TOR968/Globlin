use crate::config::Config;
use crate::diagnostics;
use crate::model::RemoveTarget;
use crate::source::{self, PackageSource};

pub fn run(config: &Config, target: &RemoveTarget) -> bool {
    match apply(config, target) {
        Ok(()) => true,
        Err(report) => {
            diagnostics::record_failures(&report);
            false
        }
    }
}

fn apply(config: &Config, target: &RemoveTarget) -> std::result::Result<(), String> {
    let sources = source::enabled(config)
        .map_err(|error| format!("{}: no package source is available: {error}\n", target.name))?;
    let source = find(&sources, target)?;
    let output = source
        .uninstall_command(&target.name)
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
    Err(describe_failure(
        target,
        &output.status.to_string(),
        &output.stdout,
        &output.stderr,
    ))
}

fn find<'a>(
    sources: &'a [Box<dyn PackageSource>],
    target: &RemoveTarget,
) -> std::result::Result<&'a dyn PackageSource, String> {
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

fn describe_failure(target: &RemoveTarget, status: &str, stdout: &[u8], stderr: &[u8]) -> String {
    format!(
        "removing {} via {} exited with {status}\n--- stdout ---\n{}\n--- stderr ---\n{}\n\n",
        target.name,
        target.source.label(),
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    )
}

#[cfg(test)]
mod tests;
