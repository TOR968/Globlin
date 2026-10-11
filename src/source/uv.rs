use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::{
    find_on_path, hidden_command, listed, not_on_path, run_listing, stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "uv";

const LATEST_PREFIX: &str = "[latest: ";

pub struct Uv {
    command: PathBuf,
}

impl Uv {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("uv"))?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> Option<String> {
        let output = run_listing(hidden_command(&self.command).args(arguments)).ok()?;
        output.status.success().then(|| stdout_text(&output))
    }
}

impl PackageSource for Uv {
    fn kind(&self) -> SourceKind {
        SourceKind::Uv
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let listing = run_listing(hidden_command(&self.command).args(["tool", "list"]))?;
        let outdated = self.run(&["tool", "list", "--outdated"]);
        listed(
            merge(&stdout_text(&listing), outdated.as_deref()),
            "uv tool list",
            &listing,
        )
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["tool", "upgrade", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["tool", "uninstall", name]);
        Some(command)
    }
}

fn merge(listing: &str, outdated: Option<&str>) -> Vec<Installed> {
    let upgrades = outdated.map(parse_upgrades);
    parse_listing(listing)
        .into_iter()
        .map(|installed| {
            let resolved = upgrades.as_ref().map(|upgrades| {
                upgrades
                    .get(&installed.name)
                    .cloned()
                    .unwrap_or_else(|| installed.version.clone())
            });
            installed.with_available(resolved)
        })
        .collect()
}

fn parse_listing(stdout: &str) -> Vec<Installed> {
    stdout
        .lines()
        .filter_map(tool_row)
        .map(|(name, version)| Installed::new(name, version, SourceKind::Uv))
        .collect()
}

fn parse_upgrades(stdout: &str) -> HashMap<String, String> {
    stdout
        .lines()
        .filter_map(|line| {
            let (name, _) = tool_row(line)?;
            Some((name.to_string(), latest(line)?))
        })
        .collect()
}

fn tool_row(line: &str) -> Option<(&str, &str)> {
    let mut fields = line.split_whitespace();
    let name = fields.next()?;
    let version = fields.next()?.strip_prefix('v')?;
    (!name.starts_with('-') && !version.is_empty()).then_some((name, version))
}

fn latest(line: &str) -> Option<String> {
    let (_, rest) = line.split_once(LATEST_PREFIX)?;
    let (version, _) = rest.split_once(']')?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_string())
}

#[cfg(test)]
mod tests;
