use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::diagnostics;
use crate::model::{Installed, SourceKind};
use crate::platform;
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "bun.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "bun";

pub struct Bun {
    command: PathBuf,
    global_dir: Option<PathBuf>,
}

impl Bun {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE)
            .or_else(default_location)
            .ok_or("bun was not found on PATH")?;
        Ok(Self {
            command,
            global_dir: resolve_global_dir(&candidates()),
        })
    }
}

impl PackageSource for Bun {
    fn kind(&self) -> SourceKind {
        SourceKind::Bun
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let Some(root) = self.global_dir.as_deref() else {
            diagnostics::record_note(&missing_root_report(&candidates()));
            return Ok(Vec::new());
        };
        let Ok(raw) = fs::read_to_string(root.join("package.json")) else {
            return Ok(Vec::new());
        };
        let names = parse_manifest(&raw)?;
        Ok(names
            .iter()
            .filter_map(|name| version_of(root, name))
            .collect())
    }

    fn update_command(&self, name: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["add", "-g", &format!("{name}@latest")]);
        command
    }

    fn uninstall_command(&self, name: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["remove", "-g", name]);
        command
    }
}

fn version_of(root: &Path, name: &str) -> Option<Installed> {
    let manifest = root.join("node_modules").join(name).join("package.json");
    let raw = fs::read_to_string(manifest).ok()?;
    let installed: InstalledManifest = serde_json::from_str(&raw).ok()?;
    Some(Installed {
        name: name.to_string(),
        version: Version::parse(&installed.version).ok()?,
        source: SourceKind::Bun,
    })
}

fn candidates() -> Vec<PathBuf> {
    candidates_from(
        std::env::var_os("BUN_INSTALL").map(PathBuf::from),
        platform::home_dir(),
    )
}

fn candidates_from(install: Option<PathBuf>, home: Option<PathBuf>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(root) = install {
        paths.push(root.join("install").join("global"));
    }
    if let Some(home) = home {
        paths.push(home.join(".bun").join("install").join("global"));
        paths.push(home);
    }
    paths
}

fn resolve_global_dir(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|root| manifest_parses(root) && has_lockfile(root))
        .or_else(|| candidates.iter().find(|root| manifest_has_dependency(root)))
        .cloned()
}

fn manifest_parses(root: &Path) -> bool {
    fs::read_to_string(root.join("package.json"))
        .ok()
        .is_some_and(|raw| parse_manifest(&raw).is_ok())
}

fn has_lockfile(root: &Path) -> bool {
    root.join("bun.lock").is_file() || root.join("bun.lockb").is_file()
}

fn manifest_has_dependency(root: &Path) -> bool {
    fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|raw| parse_manifest(&raw).ok())
        .is_some_and(|names| !names.is_empty())
}

fn missing_root_report(candidates: &[PathBuf]) -> String {
    use std::fmt::Write;

    let mut report = String::from("bun: no global package.json found; probed:\n");
    for candidate in candidates {
        writeln!(report, "  {}", candidate.display()).ok();
    }
    report.push('\n');
    report
}

fn install_root() -> PathBuf {
    if let Some(root) = std::env::var_os("BUN_INSTALL") {
        return PathBuf::from(root);
    }
    platform::home_dir().map_or_else(|| PathBuf::from(".bun"), |home| home.join(".bun"))
}

fn default_location() -> Option<PathBuf> {
    let candidate = install_root().join("bin").join(EXECUTABLE);
    candidate.is_file().then_some(candidate)
}

fn parse_manifest(raw: &str) -> Result<Vec<String>> {
    let manifest: GlobalManifest = serde_json::from_str(raw)?;
    Ok(manifest.dependencies.into_keys().collect())
}

#[derive(Deserialize)]
struct GlobalManifest {
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct InstalledManifest {
    version: String,
}

#[cfg(test)]
mod tests;
