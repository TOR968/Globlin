use std::path::PathBuf;
use std::process::Command;

use super::{find_on_path, hidden_command, node_modules_listing, PackageSource};
use crate::diagnostics;
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLES: [&str; 2] = ["yarn.cmd", "yarn.exe"];

#[cfg(not(windows))]
const EXECUTABLES: [&str; 1] = ["yarn"];

pub struct Yarn {
    command: PathBuf,
}

impl Yarn {
    pub fn new() -> Result<Self> {
        let command = EXECUTABLES
            .into_iter()
            .find_map(find_on_path)
            .ok_or("yarn was not found on PATH")?;
        Ok(Self { command })
    }

    fn global_dir(&self) -> Option<PathBuf> {
        let output = hidden_command(&self.command)
            .args(["global", "dir"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        parse_global_dir(&String::from_utf8_lossy(&output.stdout))
    }
}

impl PackageSource for Yarn {
    fn kind(&self) -> SourceKind {
        SourceKind::Yarn
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let Some(root) = self.global_dir() else {
            diagnostics::record_note(BERRY_NOTE);
            return Ok(Vec::new());
        };
        Ok(node_modules_listing(&root, SourceKind::Yarn))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["global", "add", &format!("{name}@latest")]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["global", "remove", name]);
        Some(command)
    }
}

const BERRY_NOTE: &str =
    "yarn: \"yarn global dir\" did not answer; yarn 2+ removed global installs\n\n";

const CHATTER: [&str; 6] = ["yarn ", "warning", "info", "success", "error", "Done in"];

fn parse_global_dir(stdout: &str) -> Option<PathBuf> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rfind(|line| !CHATTER.iter().any(|prefix| line.starts_with(prefix)))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests;
