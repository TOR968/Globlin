use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const SHELLS: [&str; 2] = ["pwsh.exe", "powershell.exe"];

#[cfg(not(windows))]
const SHELLS: [&str; 1] = ["pwsh"];

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
            .ok_or("no PowerShell was found on PATH")?;
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
        let output = self.shell(LISTING).output()?;
        Ok(parse_listing(&String::from_utf8_lossy(&output.stdout)))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        Some(self.shell(&format!(
            "Update-Module -Name {} -Scope CurrentUser -Force",
            quoted(name)
        )))
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
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

fn quoted(name: &str) -> String {
    format!("'{}'", name.replace('\'', "''"))
}

#[cfg(test)]
mod tests;
