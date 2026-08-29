use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "npm.cmd";

#[cfg(not(windows))]
const EXECUTABLE: &str = "npm";

pub struct Npm {
    command: PathBuf,
}

impl Npm {
    pub fn new(configured: Option<&Path>) -> Result<Self> {
        let command = resolve(configured).ok_or_else(|| {
            format!("{EXECUTABLE} was not found on PATH; set \"npm_cmd\" in the config file")
        })?;
        Ok(Self { command })
    }
}

impl PackageSource for Npm {
    fn kind(&self) -> SourceKind {
        SourceKind::Npm
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output = hidden_command(&self.command)
            .args(["ls", "-g", "--json", "--depth=0"])
            .output()?;
        parse_listing(&output.stdout)
    }

    fn update_command(&self, name: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["install", "-g", &format!("{name}@latest")]);
        command
    }

    fn uninstall_command(&self, name: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", "-g", name]);
        command
    }
}

fn resolve(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|path| path.is_file()) {
        return Some(path.to_path_buf());
    }
    find_on_path(EXECUTABLE).or_else(default_location)
}

#[cfg(windows)]
fn default_location() -> Option<PathBuf> {
    let candidate = PathBuf::from(std::env::var_os("APPDATA")?)
        .join("npm")
        .join(EXECUTABLE);
    candidate.is_file().then_some(candidate)
}

#[cfg(not(windows))]
fn default_location() -> Option<PathBuf> {
    None
}

fn parse_listing(stdout: &[u8]) -> Result<Vec<Installed>> {
    let listing: Listing = serde_json::from_slice(stdout)?;
    Ok(listing
        .dependencies
        .into_iter()
        .filter_map(to_installed)
        .collect())
}

fn to_installed((name, entry): (String, Entry)) -> Option<Installed> {
    Some(Installed {
        name,
        version: Version::parse(entry.version.as_deref()?).ok()?,
        source: SourceKind::Npm,
    })
}

#[derive(Deserialize)]
struct Listing {
    #[serde(default)]
    dependencies: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: Option<String>,
}

#[cfg(test)]
mod tests;
