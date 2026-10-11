use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{
    find_on_path, hidden_command, not_installed, pinned_to, run_listing, silent_listing,
    PackageSource,
};
use crate::model::{BlockedScript, Installed, SourceKind};
use crate::Result;

const EXECUTABLE: &str = "npm";
const PUBLIC_REGISTRY: &str = "https://registry.npmjs.org/";
const LOCKFILE: &str = ".package-lock.json";
const TRUSTED_STEMS: [&str; 1] = ["npm"];

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
        if let Some(path) = configured.filter(|path| path.is_file()) {
            if !trusted_override(path) {
                return Err(format!(
                    "\"npm_cmd\" {} was refused: it must name npm itself and must not be a downloaded file",
                    path.display()
                )
                .into());
            }
            return Ok(Self {
                command: path.to_path_buf(),
            });
        }
        let command = find_on_path(EXECUTABLE)
            .or_else(default_location)
            .ok_or_else(|| {
                not_installed(format!(
                    "{EXECUTABLE} was not found on PATH; set \"npm_cmd\" in the config file"
                ))
            })?;
        Ok(Self { command })
    }
}

impl PackageSource for Npm {
    fn kind(&self) -> SourceKind {
        SourceKind::Npm
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let output =
            run_listing(hidden_command(&self.command).args(["ls", "-g", "--json", "--depth=0"]))?;
        if output.stdout.trim_ascii().is_empty() {
            return Err(silent_listing("npm ls -g", &output).into());
        }
        let origins = self.origins();
        Ok(parse_listing(&output.stdout)?
            .into_iter()
            .map(|installed| {
                let external = origins.is_external(&installed.name);
                installed.with_external(external)
            })
            .collect())
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        self.update_to(name, "latest")
    }

    fn update_to(&self, name: &str, version: &str) -> Option<Command> {
        let configured = self.allow_scripts_config().unwrap_or_default();
        let mut command = hidden_command(&self.command);
        command.args(install_arguments(name, version, &configured));
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
        let output = run_listing(hidden_command(&self.command).args([
            "config",
            "set",
            &format!("allow-scripts={merged}"),
            "--location=user",
        ]))?;
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
    fn origins(&self) -> Origins {
        let root = self.answer(&["root", "-g"]).map(PathBuf::from);
        let registry = self.answer(&["config", "get", "registry"]);
        let lock = root
            .as_deref()
            .and_then(|root| std::fs::read_to_string(root.join(LOCKFILE)).ok());
        Origins {
            root,
            registry,
            lock: lock.as_deref().and_then(parse_lock).unwrap_or_default(),
        }
    }

    fn answer(&self, arguments: &[&str]) -> Option<String> {
        let output = run_listing(hidden_command(&self.command).args(arguments)).ok()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (output.status.success() && !text.is_empty()).then_some(text)
    }

    fn allow_scripts_config(&self) -> Result<String> {
        let output = run_listing(hidden_command(&self.command).args([
            "config",
            "get",
            "allow-scripts",
            "-g",
        ]))?;
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

fn install_arguments(name: &str, version: &str, configured_allow_scripts: &str) -> Vec<String> {
    let pinned = pinned_to(name, version);
    vec![
        "install".to_owned(),
        "-g".to_owned(),
        pinned.clone(),
        format!(
            "--allow-scripts={}",
            merge_allow_list(configured_allow_scripts, [pinned])
        ),
        "--strict-allow-scripts".to_owned(),
    ]
}

#[derive(Default)]
struct Origins {
    root: Option<PathBuf>,
    registry: Option<String>,
    lock: BTreeMap<String, LockEntry>,
}

impl Origins {
    fn is_external(&self, name: &str) -> bool {
        match self.lock.get(&format!("node_modules/{name}")) {
            Some(entry) if entry.link => true,
            Some(LockEntry {
                resolved: Some(resolved),
                ..
            }) => !self.resolves_through_registry(resolved),
            Some(_) => false,
            None => self
                .root
                .as_deref()
                .is_some_and(|root| is_link(&root.join(name))),
        }
    }

    fn resolves_through_registry(&self, resolved: &str) -> bool {
        resolved.starts_with(PUBLIC_REGISTRY)
            || self.registry.as_deref().is_some_and(|registry| {
                let registry = registry.trim_end_matches('/');
                resolved
                    .strip_prefix(registry)
                    .is_some_and(|rest| rest.starts_with('/'))
            })
    }
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

fn parse_lock(raw: &str) -> Option<BTreeMap<String, LockEntry>> {
    let lock: Lock = serde_json::from_str(raw).ok()?;
    Some(lock.packages)
}

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    packages: BTreeMap<String, LockEntry>,
}

#[derive(Deserialize)]
struct LockEntry {
    resolved: Option<String>,
    #[serde(default)]
    link: bool,
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

fn trusted_override(path: &Path) -> bool {
    names_npm(path) && !downloaded(path)
}

fn names_npm(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            TRUSTED_STEMS
                .iter()
                .any(|trusted| stem.eq_ignore_ascii_case(trusted))
        })
}

#[cfg(windows)]
fn downloaded(path: &Path) -> bool {
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    Path::new(&stream).exists()
}

#[cfg(not(windows))]
fn downloaded(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn default_location() -> Option<PathBuf> {
    let candidate = PathBuf::from(std::env::var_os("APPDATA")?)
        .join("npm")
        .join("npm.cmd");
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
