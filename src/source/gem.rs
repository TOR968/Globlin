use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use super::{
    find_on_path, hidden_command, listed, not_on_path, run_listing, stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::platform;
use crate::Result;

const EXECUTABLE: &str = "gem";

const DEFAULT_MARKER: &str = "default: ";

const SPECIFICATIONS: &str = "specifications";

const DEFAULT_SPECIFICATIONS: &str = "default";

const SPECIFICATION_SUFFIX: &str = ".gemspec";

const USER_INSTALL: &str = "--user-install";

pub struct Gem {
    command: PathBuf,
    user_install: OnceLock<bool>,
}

impl Gem {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("gem"))?;
        Ok(Self {
            command,
            user_install: OnceLock::new(),
        })
    }

    fn environment(&self, item: &str) -> Option<String> {
        let text = self.run(&["environment", item])?;
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    fn installs_for_the_user(&self) -> bool {
        *self.user_install.get_or_init(|| {
            self.environment("gemdir")
                .is_some_and(|home| !platform::writable(Path::new(&home)))
        })
    }

    fn within_reach(&self, installed: Vec<Installed>) -> Vec<Installed> {
        let Some(paths) = self.environment("gempath") else {
            return installed;
        };
        let (open, shut): (Vec<PathBuf>, Vec<PathBuf>) =
            std::env::split_paths(&paths).partition(|path| platform::writable(path));
        if shut.is_empty() {
            return installed;
        }
        owned(installed, &specifications(&open))
    }

    fn run(&self, arguments: &[&str]) -> Option<String> {
        let output = run_listing(hidden_command(&self.command).args(arguments)).ok()?;
        output.status.success().then(|| stdout_text(&output))
    }
}

impl PackageSource for Gem {
    fn kind(&self) -> SourceKind {
        SourceKind::Gem
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let listing = run_listing(hidden_command(&self.command).args(["list", "--local"]))?;
        let outdated = self.run(&["outdated"]);
        let installed = listed(
            merge(&stdout_text(&listing), outdated.as_deref()),
            "gem list --local",
            &listing,
        )?;
        Ok(self.within_reach(installed))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["update", name]);
        if self.installs_for_the_user() {
            command.arg(USER_INSTALL);
        }
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

fn specifications(homes: &[PathBuf]) -> HashSet<String> {
    homes
        .iter()
        .map(|home| home.join(SPECIFICATIONS))
        .flat_map(|directory| [directory.join(DEFAULT_SPECIFICATIONS), directory])
        .filter_map(|directory| fs::read_dir(directory).ok())
        .flat_map(|entries| entries.filter_map(std::result::Result::ok))
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            name.strip_suffix(SPECIFICATION_SUFFIX).map(str::to_string)
        })
        .collect()
}

fn owned(installed: Vec<Installed>, specifications: &HashSet<String>) -> Vec<Installed> {
    installed
        .into_iter()
        .filter(|gem| {
            let exact = format!("{}-{}", gem.name, gem.version);
            let platformed = format!("{exact}-");
            specifications
                .iter()
                .any(|stem| *stem == exact || stem.starts_with(&platformed))
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
