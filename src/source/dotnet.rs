use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "dotnet.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "dotnet";

pub struct Dotnet {
    command: PathBuf,
}

impl Dotnet {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("dotnet was not found on PATH")?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> String {
        hidden_command(&self.command)
            .args(arguments)
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default()
    }
}

impl PackageSource for Dotnet {
    fn kind(&self) -> SourceKind {
        SourceKind::Dotnet
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let reported = parse_json(&self.run(&["tool", "list", "--global", "--format", "json"]));
        if reported.is_empty() {
            return Ok(parse_table(&self.run(&["tool", "list", "--global"])));
        }
        Ok(reported)
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["tool", "update", "--global", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["tool", "uninstall", "--global", name]);
        Some(command)
    }
}

fn parse_json(stdout: &str) -> Vec<Installed> {
    let Ok(listing) = serde_json::from_str::<Listing>(stdout) else {
        return Vec::new();
    };
    listing
        .data
        .into_iter()
        .filter(|entry| !entry.package_id.is_empty() && !entry.version.is_empty())
        .map(|entry| Installed::new(entry.package_id, entry.version, SourceKind::Dotnet))
        .collect()
}

fn parse_table(stdout: &str) -> Vec<Installed> {
    let mut lines = stdout.lines().skip_while(|line| !is_separator(line));
    if lines.next().is_none() {
        return Vec::new();
    }
    lines
        .map(|line| line.split_whitespace().collect::<Vec<&str>>())
        .filter(|row| row.len() >= 2)
        .map(|row| Installed::new(row[0], row[1], SourceKind::Dotnet))
        .collect()
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|glyph| glyph == '-' || glyph == ' ')
}

#[derive(Deserialize)]
struct Listing {
    #[serde(alias = "Data")]
    data: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    #[serde(alias = "PackageId")]
    package_id: String,
    #[serde(alias = "Version")]
    version: String,
}

#[cfg(test)]
mod tests;
