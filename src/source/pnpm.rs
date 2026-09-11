use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLES: [&str; 2] = ["pnpm.cmd", "pnpm.exe"];

#[cfg(not(windows))]
const EXECUTABLES: [&str; 1] = ["pnpm"];

pub struct Pnpm {
    command: PathBuf,
}

impl Pnpm {
    pub fn new() -> Result<Self> {
        let command = EXECUTABLES
            .into_iter()
            .find_map(find_on_path)
            .ok_or("pnpm was not found on PATH")?;
        Ok(Self { command })
    }
}

impl PackageSource for Pnpm {
    fn kind(&self) -> SourceKind {
        SourceKind::Pnpm
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = hidden_command(&self.command)
            .args(["ls", "-g", "--json", "--depth=0"])
            .output()?;
        Ok(parse_listing(&String::from_utf8_lossy(&output.stdout)))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["add", "-g", &format!("{name}@latest")]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["remove", "-g", name]);
        Some(command)
    }
}

fn parse_listing(stdout: &str) -> Vec<Installed> {
    let Ok(listing) = serde_json::from_str::<Listing>(stdout.trim_start_matches('\u{feff}')) else {
        return Vec::new();
    };
    let projects = match listing {
        Listing::Many(projects) => projects,
        Listing::One(project) => vec![project],
    };
    let mut installed: Vec<Installed> = projects
        .into_iter()
        .flat_map(|project| project.dependencies)
        .filter_map(to_installed)
        .collect();
    installed.sort_by(|left, right| left.name.cmp(&right.name));
    installed.dedup_by(|left, right| left.name == right.name);
    installed
}

fn to_installed((name, entry): (String, Entry)) -> Option<Installed> {
    let version = entry.version?;
    semver::Version::parse(&version).ok()?;
    Some(Installed::new(name, version, SourceKind::Pnpm))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Listing {
    Many(Vec<Project>),
    One(Project),
}

#[derive(Deserialize)]
struct Project {
    #[serde(default)]
    dependencies: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: Option<String>,
}

#[cfg(test)]
mod tests;
