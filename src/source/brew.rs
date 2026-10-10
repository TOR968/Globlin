use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "brew";

pub struct Brew {
    command: PathBuf,
}

impl Brew {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("brew was not found on PATH")?;
        Ok(Self { command })
    }

    fn brew(&self) -> Command {
        let mut command = hidden_command(&self.command);
        command.env("HOMEBREW_NO_AUTO_UPDATE", "1");
        command
    }
}

impl PackageSource for Brew {
    fn kind(&self) -> SourceKind {
        SourceKind::Brew
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = self
            .brew()
            .args(["info", "--json=v2", "--installed"])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "brew info failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        parse_info(&output.stdout)
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = self.brew();
        command.args(["upgrade", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = self.brew();
        command.args(["uninstall", name]);
        Some(command)
    }
}

#[derive(Deserialize)]
struct Info {
    #[serde(default)]
    formulae: Vec<Formula>,
    #[serde(default)]
    casks: Vec<Cask>,
}

#[derive(Deserialize)]
struct Formula {
    name: String,
    versions: FormulaVersions,
    #[serde(default)]
    revision: u32,
    #[serde(default)]
    installed: Vec<Keg>,
    #[serde(default)]
    outdated: bool,
}

#[derive(Deserialize)]
struct FormulaVersions {
    stable: Option<String>,
}

#[derive(Deserialize)]
struct Keg {
    version: String,
}

#[derive(Deserialize)]
struct Cask {
    token: String,
    version: String,
    installed: Option<String>,
    #[serde(default)]
    outdated: bool,
}

fn parse_info(stdout: &[u8]) -> Result<Vec<Installed>> {
    let info: Info = serde_json::from_slice(stdout)?;
    let formulae = info.formulae.into_iter().filter_map(formula);
    let casks = info.casks.into_iter().filter_map(cask);
    Ok(formulae.chain(casks).collect())
}

fn formula(formula: Formula) -> Option<Installed> {
    let version = formula.installed.last()?.version.clone();
    let available = if formula.outdated {
        formula
            .versions
            .stable
            .map(|stable| match formula.revision {
                0 => stable,
                revision => format!("{stable}_{revision}"),
            })
    } else {
        Some(version.clone())
    };
    Some(Installed::new(formula.name, version, SourceKind::Brew).with_available(available))
}

fn cask(cask: Cask) -> Option<Installed> {
    let version = cask.installed?;
    let available = if cask.outdated {
        cask.version
    } else {
        version.clone()
    };
    Some(Installed::new(cask.token, version, SourceKind::Brew).with_available(Some(available)))
}

#[cfg(test)]
mod tests;
