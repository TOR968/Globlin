use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "gem.cmd";

#[cfg(not(windows))]
const EXECUTABLE: &str = "gem";

const DEFAULT_MARKER: &str = "default: ";

pub struct Gem {
    command: PathBuf,
}

impl Gem {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("gem was not found on PATH")?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> Option<String> {
        let output = hidden_command(&self.command)
            .args(arguments)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

impl PackageSource for Gem {
    fn kind(&self) -> SourceKind {
        SourceKind::Gem
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let listing = self.run(&["list", "--local"]).unwrap_or_default();
        let outdated = self.run(&["outdated"]);
        Ok(merge(&listing, outdated.as_deref()))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["update", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", name, "--all", "--executables"]);
        Some(command)
    }
}

fn merge(listing: &str, outdated: Option<&str>) -> Vec<Installed> {
    let upgrades = outdated.map(parse_outdated);
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
        .filter_map(parenthesised)
        .filter_map(|(name, versions)| {
            let newest = versions.split(", ").next()?;
            let newest = newest.strip_prefix(DEFAULT_MARKER).unwrap_or(newest);
            let version = newest.split_whitespace().next()?;
            Some(Installed::new(name, version, SourceKind::Gem))
        })
        .collect()
}

fn parse_outdated(stdout: &str) -> HashMap<String, String> {
    stdout
        .lines()
        .filter_map(parenthesised)
        .filter_map(|(name, versions)| {
            let (current, remote) = versions.split_once(" < ")?;
            let latest = remote.split(", ").next()?.trim();
            (!latest.is_empty() && latest != current.trim())
                .then(|| (name.to_string(), latest.to_string()))
        })
        .collect()
}

fn parenthesised(line: &str) -> Option<(&str, &str)> {
    let (name, rest) = line.trim().split_once(" (")?;
    let inner = rest.strip_suffix(')')?;
    (!name.is_empty() && !name.contains(' ') && !inner.is_empty()).then_some((name, inner))
}

#[cfg(test)]
mod tests;
