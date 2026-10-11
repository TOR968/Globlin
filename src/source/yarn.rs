use std::path::PathBuf;
use std::process::Command;

use super::{
    find_on_path, hidden_command, node_modules_listing, not_on_path, pinned_to, run_listing,
    stdout_text, PackageSource,
};
use crate::diagnostics;
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "yarn";

pub struct Yarn {
    command: PathBuf,
}

impl Yarn {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or_else(|| not_on_path("yarn"))?;
        Ok(Self { command })
    }

    fn global_dir(&self) -> Option<PathBuf> {
        let output = run_listing(hidden_command(&self.command).args(["global", "dir"])).ok()?;
        if !output.status.success() {
            return None;
        }
        parse_global_dir(&stdout_text(&output))
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
        self.update_to(name, "latest")
    }

    fn update_to(&self, name: &str, version: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["global", "add", &pinned_to(name, version)]);
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
