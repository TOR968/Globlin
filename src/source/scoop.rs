use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "scoop.cmd";

#[cfg(not(windows))]
const EXECUTABLE: &str = "scoop";

pub struct Scoop {
    command: PathBuf,
    root: PathBuf,
}

impl Scoop {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE).ok_or("scoop was not found on PATH")?;
        let root = root().ok_or("the scoop directory has no apps to read")?;
        Ok(Self { command, root })
    }
}

impl PackageSource for Scoop {
    fn kind(&self) -> SourceKind {
        SourceKind::Scoop
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        Ok(listing(&self.root))
    }

    fn update_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["update", name]);
        Some(command)
    }

    fn uninstall_command(&self, name: &str) -> Option<Command> {
        let mut command = hidden_command(&self.command);
        command.args(["uninstall", name]);
        Some(command)
    }
}

fn root() -> Option<PathBuf> {
    let root = match std::env::var_os("SCOOP") {
        Some(configured) => PathBuf::from(configured),
        None => PathBuf::from(std::env::var_os("USERPROFILE")?).join("scoop"),
    };
    root.join("apps").is_dir().then_some(root)
}

fn listing(root: &Path) -> Vec<Installed> {
    let Ok(entries) = fs::read_dir(root.join("apps")) else {
        return Vec::new();
    };
    let mut installed: Vec<Installed> = entries
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| app(root, &entry.file_name().to_string_lossy()))
        .collect();
    installed.sort_by(|left, right| left.name.cmp(&right.name));
    installed
}

fn app(root: &Path, name: &str) -> Option<Installed> {
    let current = root.join("apps").join(name).join("current");
    let version = read(&current.join("manifest.json"), version_of)?;
    let indexed = read(&current.join("install.json"), bucket_of)
        .and_then(|bucket| indexed_version(root, &bucket, name));
    Some(Installed::new(name, version, SourceKind::Scoop).with_available(indexed))
}

fn indexed_version(root: &Path, bucket: &str, name: &str) -> Option<String> {
    let bucket = root.join("buckets").join(bucket);
    let manifest = format!("{name}.json");
    read(&bucket.join("bucket").join(&manifest), version_of)
        .or_else(|| read(&bucket.join(&manifest), version_of))
}

fn read(path: &Path, parse: fn(&str) -> Option<String>) -> Option<String> {
    parse(&fs::read_to_string(path).ok()?)
}

fn version_of(raw: &str) -> Option<String> {
    let manifest: Manifest = serde_json::from_str(raw).ok()?;
    (!manifest.version.is_empty()).then_some(manifest.version)
}

fn bucket_of(raw: &str) -> Option<String> {
    let record: InstallRecord = serde_json::from_str(raw).ok()?;
    record.bucket.filter(|bucket| !bucket.is_empty())
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
}

#[derive(Deserialize)]
struct InstallRecord {
    bucket: Option<String>,
}

#[cfg(test)]
mod tests;
