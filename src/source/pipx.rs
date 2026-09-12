use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "pipx.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "pipx";

pub struct Pipx {
    command: PathBuf,
}

impl Pipx {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("pipx was not found on PATH")?;
        Ok(Self { command })
    }
}

impl PackageSource for Pipx {
    fn kind(&self) -> SourceKind {
        SourceKind::Pipx
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = hidden_command(&self.command)
            .args(["list", "--short"])
            .output()?;
        Ok(parse_listing(&String::from_utf8_lossy(&output.stdout)))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["upgrade", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", name]);
        Some(command)
    }
}

fn parse_listing(stdout: &str) -> Vec<Installed> {
    stdout
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<&str>>())
        .filter(|row| row.len() == 2)
        .map(|row| Installed::new(row[0], row[1], SourceKind::Pipx))
        .collect()
}

#[cfg(test)]
mod tests;
