use std::path::PathBuf;
use std::process::Command;

use super::{
    find_on_path, hidden_command, listed, not_on_path, run_listing, stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "pipx";

pub struct Pipx {
    command: PathBuf,
}

impl Pipx {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("pipx"))?;
        Ok(Self { command })
    }
}

impl PackageSource for Pipx {
    fn kind(&self) -> SourceKind {
        SourceKind::Pipx
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = run_listing(hidden_command(&self.command).args(["list", "--short"]))?;
        listed(parse_listing(&stdout_text(&output)), "pipx list", &output)
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
