use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{BlockedScript, Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "npm.cmd";

#[cfg(not(windows))]
const EXECUTABLE: &str = "npm";

const STRICT_REFUSAL_CODE: &str = "ESTRICTALLOWSCRIPTS";
const BLOCKED_LINE_PREFIX: &str = "npm error   ";
const HEADER_PREFIX: &str = "npm error --strict-allow-scripts: ";
const HEADER_SUFFIX: &str = " package(s) have install scripts not covered by allowScripts:";
const REMEDIATION_PREFIX: &str = "npm error Allow them with";

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

    fn update_command(&self, name: &str) -> Option<Command> {
        let configured = self.allow_scripts_config().unwrap_or_default();
        let mut command = hidden_command(&self.command);
        command.args(install_arguments(name, &configured));
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", "-g", name]);
        Some(command)
    }

    fn blocked_scripts(&self, stderr: &str) -> Option<Vec<BlockedScript>> {
        parse_blocked_scripts(stderr)
    }

    fn approve_scripts(&self, scripts: &[BlockedScript]) -> Result<()> {
        let configured = self.allow_scripts_config()?;
        let merged = merge_allow_list(&configured, scripts.iter().map(BlockedScript::package));
        let output = hidden_command(&self.command)
            .args([
                "config",
                "set",
                &format!("allow-scripts={merged}"),
                "--location=user",
            ])
            .output()?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "npm config set allow-scripts failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into())
    }
}

impl Npm {
    fn allow_scripts_config(&self) -> Result<String> {
        let output = hidden_command(&self.command)
            .args(["config", "get", "allow-scripts", "-g"])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "npm config get allow-scripts failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn install_arguments(name: &str, configured_allow_scripts: &str) -> Vec<String> {
    vec![
        "install".to_owned(),
        "-g".to_owned(),
        format!("{name}@latest"),
        format!(
            "--allow-scripts={}",
            merge_allow_list(configured_allow_scripts, [name.to_owned()])
        ),
        "--strict-allow-scripts".to_owned(),
    ]
}

fn merge_allow_list(configured: &str, additions: impl IntoIterator<Item = String>) -> String {
    let mut entries: Vec<String> = configured
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|entry| !entry.is_empty() && *entry != "undefined" && *entry != "null")
        .map(str::to_owned)
        .collect();
    for addition in additions {
        if !entries.contains(&addition) {
            entries.push(addition);
        }
    }
    entries.join(",")
}

fn parse_blocked_scripts(stderr: &str) -> Option<Vec<BlockedScript>> {
    if !stderr.contains(STRICT_REFUSAL_CODE) {
        return None;
    }
    let mut lines = stderr.lines().map(str::trim_end);
    let expected = lines.by_ref().find_map(header_count)?;
    let blocked: Vec<BlockedScript> = lines
        .take_while(|line| !line.starts_with(REMEDIATION_PREFIX))
        .map(blocked_line)
        .collect::<Option<_>>()?;
    if blocked.is_empty() || blocked.len() != expected || has_duplicate_name(&blocked) {
        return None;
    }
    Some(blocked)
}

fn header_count(line: &str) -> Option<usize> {
    line.strip_prefix(HEADER_PREFIX)?
        .strip_suffix(HEADER_SUFFIX)?
        .parse()
        .ok()
}

fn has_duplicate_name(blocked: &[BlockedScript]) -> bool {
    let mut names: Vec<&str> = blocked.iter().map(|entry| entry.name.as_str()).collect();
    names.sort_unstable();
    names.windows(2).any(|pair| pair[0] == pair[1])
}

fn blocked_line(line: &str) -> Option<BlockedScript> {
    let entry = line.strip_prefix(BLOCKED_LINE_PREFIX)?;
    let (label, rest) = entry.split_once(" (")?;
    let scripts = rest.strip_suffix(')')?;
    let (name, version) = match label.rfind('@') {
        Some(at) if at > 0 => (&label[..at], &label[at + 1..]),
        _ => (label, ""),
    };
    Some(BlockedScript {
        name: name.to_owned(),
        version: version.to_owned(),
        scripts: scripts.to_owned(),
    })
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
    let version = entry.version?;
    semver::Version::parse(&version).ok()?;
    Some(Installed::new(name, version, SourceKind::Npm))
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
