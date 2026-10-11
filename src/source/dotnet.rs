use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::{
    find_on_path, hidden_command, listed, not_on_path, run_listing, stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "dotnet";
const NO_SDK: &str = "No .NET SDKs were found";

pub struct Dotnet {
    command: PathBuf,
}

impl Dotnet {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("dotnet"))?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> Result<std::process::Output> {
        run_listing(hidden_command(&self.command).args(arguments))
    }
}

impl PackageSource for Dotnet {
    fn kind(&self) -> SourceKind {
        SourceKind::Dotnet
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let json = self.run(&["tool", "list", "--global", "--format", "json"])?;
        let reported = parse_json(&stdout_text(&json));
        if !reported.is_empty() {
            return Ok(reported);
        }
        let table = self.run(&["tool", "list", "--global"])?;
        if without_sdk(&table) {
            return Ok(Vec::new());
        }
        listed(
            parse_table(&stdout_text(&table)),
            "dotnet tool list --global",
            &table,
        )
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

fn without_sdk(output: &std::process::Output) -> bool {
    [&output.stdout, &output.stderr]
        .into_iter()
        .any(|stream| String::from_utf8_lossy(stream).contains(NO_SDK))
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
