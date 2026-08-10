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
mod tests {
    use super::*;

    const LISTING: &[u8] = br#"{
      "name": "npm",
      "problems": ["invalid: @kilocode/cli@ C:\\Users\\x\\AppData\\Roaming\\npm\\node_modules\\@kilocode"],
      "dependencies": {
        "prettier": { "version": "3.9.6", "resolved": "https://registry.npmjs.org/prettier/-/prettier-3.9.6.tgz" },
        "@salesforce/cli": { "version": "2.145.6" },
        "vanished": { "required": "^1.0.0", "missing": true },
        "not-semver": { "version": "latest" }
      }
    }"#;

    #[test]
    fn parses_scoped_and_plain_packages() {
        let installed = parse_listing(LISTING).unwrap();
        let names: Vec<&str> = installed.iter().map(|item| item.name.as_str()).collect();

        assert_eq!(names, vec!["@salesforce/cli", "prettier"]);
    }

    #[test]
    fn skips_entries_without_a_usable_version() {
        let installed = parse_listing(LISTING).unwrap();

        assert!(!installed.iter().any(|item| item.name == "vanished"));
        assert!(!installed.iter().any(|item| item.name == "not-semver"));
    }

    #[test]
    fn reads_the_version_and_tags_the_source() {
        let installed = parse_listing(LISTING).unwrap();
        let prettier = installed
            .iter()
            .find(|item| item.name == "prettier")
            .unwrap();

        assert_eq!(prettier.version, Version::parse("3.9.6").unwrap());
        assert_eq!(prettier.source, SourceKind::Npm);
    }

    #[test]
    fn a_listing_without_dependencies_is_empty_not_an_error() {
        let installed = parse_listing(br#"{"name":"npm"}"#).unwrap();
        assert!(installed.is_empty());
    }

    #[test]
    fn unparseable_output_is_an_error() {
        assert!(parse_listing(b"").is_err());
        assert!(parse_listing(b"npm ERR! code ENOENT").is_err());
    }
}
