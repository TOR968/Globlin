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
    remove_from(&sources, target)
}

fn remove_from(
    sources: &[Box<dyn PackageSource>],
    target: &RemoveTarget,
) -> std::result::Result<(), String> {
    let source = source::source_for(sources, &target.name, target.source)?;
    if let Some(removed) = source.remove_directly(&target.name) {
        return removed.map_err(|error| {
            format!(
                "removing {} via {} failed: {error}\n",
                target.name,
                target.source.label()
            )
        });
    }
    let output = source::run_change(
        source.uninstall_command(&target.name),
        &target.name,
        target.source,
    )?;

    if output.status.success() {
        return Ok(());
    }
    Err(diagnostics::command_report(
        &format!("removing {} via {}", target.name, target.source.label()),
        &output,
    ))
}

#[cfg(test)]
mod tests;
