use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use semver::Version;
use serde::Deserialize;

use super::{find_on_path, hidden_command, PackageSource};
use crate::model::{Installed, SourceKind};
use crate::platform;
use crate::Result;

#[cfg(windows)]
const EXECUTABLE: &str = "bun.exe";

#[cfg(not(windows))]
const EXECUTABLE: &str = "bun";

pub struct Bun {
    command: PathBuf,
    global_dir: PathBuf,
}

impl Bun {
    pub fn new() -> Result<Self> {
        let command = find_on_path(EXECUTABLE)
            .or_else(default_location)
            .ok_or("bun was not found on PATH")?;
        Ok(Self {
            command,
            global_dir: global_dir(),
        })
    }

    fn version_of(&self, name: &str) -> Option<Installed> {
        let manifest = self
            .global_dir
            .join("node_modules")
            .join(name)
            .join("package.json");
        let raw = fs::read_to_string(manifest).ok()?;
        let installed: InstalledManifest = serde_json::from_str(&raw).ok()?;
        Some(Installed {
            name: name.to_string(),
            version: Version::parse(&installed.version).ok()?,
            source: SourceKind::Bun,
        })
    }
}

impl PackageSource for Bun {
    fn kind(&self) -> SourceKind {
        SourceKind::Bun
    }

    fn installed(&self) -> Result<Vec<Installed>> {
        let Ok(raw) = fs::read_to_string(self.global_dir.join("package.json")) else {
            return Ok(Vec::new());
        };
        let names = parse_manifest(&raw)?;
        Ok(names
            .iter()
            .filter_map(|name| self.version_of(name))
            .collect())
    }

    fn update_command(&self, name: &str) -> Command {
        let mut command = hidden_command(&self.command);
        command.args(["add", "-g", &format!("{name}@latest")]);
        command
    }
}

fn global_dir() -> PathBuf {
    install_root().join("install").join("global")
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
mod tests {
    use super::*;

    #[test]
    fn reads_dependency_names_from_the_global_manifest() {
        let raw = r#"{"dependencies":{"opencode-ai":"1.17.9","@scope/tool":"^2.0.0"}}"#;
        assert_eq!(
            parse_manifest(raw).unwrap(),
            vec!["@scope/tool".to_string(), "opencode-ai".to_string()]
        );
    }

    #[test]
    fn an_empty_global_store_yields_no_packages() {
        assert!(parse_manifest("{}").unwrap().is_empty());
        assert!(parse_manifest(r#"{"dependencies":{}}"#).unwrap().is_empty());
    }

    #[test]
    fn a_malformed_manifest_is_an_error() {
        assert!(parse_manifest("not json").is_err());
    }

    #[test]
    fn the_global_dir_sits_under_the_install_root() {
        let dir = global_dir();
        assert!(dir.ends_with("install/global") || dir.ends_with("install\\global"));
    }
}
