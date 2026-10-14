use std::path::PathBuf;
use std::process::Command;

use super::{
    find_on_path, hidden_command, listed, not_installed, plain_name, quoted, run_listing,
    stdout_text, PackageSource,
};
use crate::model::{Installed, SourceKind};
use crate::Result;

const SHELLS: [&str; 2] = ["pwsh", "powershell"];

const LISTING: &str =
    "Get-InstalledModule | ForEach-Object { $_.Name + ' ' + $_.Version.ToString() }";

pub struct PsGallery {
    command: PathBuf,
}

impl PsGallery {
    pub fn new() -> Result<Self> {
        let command = SHELLS
            .into_iter()
            .find_map(find_on_path)
            .ok_or_else(|| not_installed("no PowerShell was found on PATH".to_string()))?;
        Ok(Self { command })
    }

    fn shell(&self, script: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        command
    }
}

impl PackageSource for PsGallery {
    fn kind(&self) -> SourceKind {
        SourceKind::PsGallery
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = run_listing(&mut self.shell(LISTING))?;
        listed(
            parse_listing(&stdout_text(&output)),
            "Get-InstalledModule",
            &output,
        )
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let name = plain_name(name)?;
        Some(self.shell(&format!(
            "Update-Module -Name {} -Scope CurrentUser -Force",
            quoted(name)
        )))
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let name = plain_name(name)?;
        Some(self.shell(&format!("Uninstall-Module -Name {}", quoted(name))))
    }
}

fn parse_listing(stdout: &str) -> Vec<Installed> {
    stdout
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<&str>>())
        .filter(|row| row.len() == 2)
        .map(|row| Installed::new(row[0], row[1], SourceKind::PsGallery))
        .collect()
}

#[cfg(test)]
mod tests;
