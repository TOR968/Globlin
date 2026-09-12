use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "cargo.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "cargo";

pub struct Cargo {
    command: PathBuf,
}

impl Cargo {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("cargo was not found on PATH")?;
        Ok(Self { command })
    }
}

impl PackageSource for Cargo {
    fn kind(&self) -> SourceKind {
        SourceKind::Cargo
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = hidden_command(&self.command)
            .args(["install", "--list"])
            .output()?;
        Ok(parse_listing(&String::from_utf8_lossy(&output.stdout)))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["install", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", name]);
        Some(command)
    }
}

fn parse_listing(stdout: &str) -> Vec<Installed> {
    stdout.lines().filter_map(to_installed).collect()
}

fn to_installed(line: &str) -> Option<Installed> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (name, version) = line.strip_suffix(':')?.split_once(" v")?;
    semver::Version::parse(version).ok()?;
    Some(Installed::new(name, version, SourceKind::Cargo))
}

#[cfg(test)]
mod tests;
