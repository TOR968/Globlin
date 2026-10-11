use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::{
    find_on_path, hidden_command, listed, not_on_path, run_listing, stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "choco";

const LOCAL_ONLY_UNTIL: u64 = 2;

pub struct Choco {
    command: PathBuf,
}

impl Choco {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("choco"))?;
        Ok(Self { command })
    }

    fn run(&self, arguments: &[&str]) -> String {
        run_listing(hidden_command(&self.command).args(arguments))
            .map(|output| stdout_text(&output))
            .unwrap_or_default()
    }

    fn list_arguments(&self) -> Vec<&'static str> {
        if major_version(&self.run(&["--version"])).unwrap_or(LOCAL_ONLY_UNTIL) < LOCAL_ONLY_UNTIL {
            vec!["list", "-r", "--local-only"]
        } else {
            vec!["list", "-r"]
        }
    }
}

impl PackageSource for Choco {
    fn kind(&self) -> SourceKind {
        SourceKind::Choco
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let listing = run_listing(hidden_command(&self.command).args(self.list_arguments()))?;
        let outdated = self.run(&["outdated", "-r", "--ignore-pinned"]);
        listed(
            merge(&stdout_text(&listing), &outdated),
            "choco list",
            &listing,
        )
    }

    fn update_command(&self, _name: &str) -> Option<Command> {
        None
    }

    fn uninstall_command(&self, _name: &str) -> Option<Command> {
        None
    }
}

fn major_version(raw: &str) -> Option<u64> {
    raw.trim()
        .lines()
        .next_back()?
        .trim()
        .split('.')
        .next()?
        .parse()
        .ok()
}

fn merge(listing: &str, outdated: &str) -> Vec<Installed> {
    let upgrades = parse_outdated(outdated);
    parse_listing(listing)
        .into_iter()
        .map(|installed| {
            let available = upgrades.get(&installed.name).cloned();
            installed.with_available(available)
        })
        .collect()
}

fn parse_listing(raw: &str) -> Vec<Installed> {
    raw.lines()
        .filter_map(fields)
        .filter(|row| row.len() == 2)
        .map(|row| Installed::new(row[0], row[1], SourceKind::Choco))
        .collect()
}

fn parse_outdated(raw: &str) -> HashMap<String, String> {
    raw.lines()
        .filter_map(fields)
        .filter(|row| row.len() >= 3 && row[1] != row[2])
        .map(|row| (row[0].to_string(), row[2].to_string()))
        .collect()
}

fn fields(line: &str) -> Option<Vec<&str>> {
    let row: Vec<&str> = line.trim().split('|').collect();
    (row.len() >= 2 && !row[0].is_empty() && !row[1].is_empty()).then_some(row)
}

#[cfg(test)]
mod tests;
