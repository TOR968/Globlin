use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::config::Config;
use crate::model::{BlockedScript, Installed, SourceKind, KINDS};
use crate::Result;

mod bun;
mod cargo;
mod choco;
mod dotnet;
mod gem;
mod golang;
mod npm;
mod pipx;
mod pnpm;
mod psgallery;
mod scoop;
mod uv;
mod winget;
mod yarn;

pub use bun::Bun;
pub use cargo::Cargo;
pub use choco::Choco;
pub use dotnet::Dotnet;
pub use gem::Gem;
pub use golang::Go;
pub use npm::Npm;
pub use pipx::Pipx;
pub use pnpm::Pnpm;
pub use psgallery::PsGallery;
pub use scoop::Scoop;
pub use uv::Uv;
pub use winget::Winget;
pub use yarn::Yarn;

pub trait PackageSource {
    fn kind(&self) -> SourceKind;

    fn installed(&self) -> Result<Vec<Installed>>;

    fn update_command(&self, name: &str) -> Option<Command>;

    fn uninstall_command(&self, name: &str) -> Option<Command>;

    fn blocked_scripts(&self, _stderr: &str) -> Option<Vec<BlockedScript>> {
        None
    }

    fn approve_scripts(&self, _scripts: &[BlockedScript]) -> Result<()> {
        Err(format!("{} does not gate install scripts", self.kind().label()).into())
    }
}

pub fn enabled(config: &Config) -> Result<Vec<Box<dyn PackageSource>>> {
    let mut sources: Vec<Box<dyn PackageSource>> = Vec::new();
    let mut failures = Vec::new();

    for kind in KINDS {
        if !config.source_enabled(kind) {
            continue;
        }
        match build(kind, config) {
            Ok(source) => sources.push(source),
            Err(error) => failures.push(error.to_string()),
        }
    }

    if sources.is_empty() {
        return Err(if failures.is_empty() {
            "no package sources are enabled in the config".into()
        } else {
            failures.join("; ").into()
        });
    }
    Ok(sources)
}

fn build(kind: SourceKind, config: &Config) -> Result<Box<dyn PackageSource>> {
    Ok(match kind {
        SourceKind::Npm => Box::new(Npm::new(config.npm_cmd.as_deref())?),
        SourceKind::Bun => Box::new(Bun::new()?),
        SourceKind::Pnpm => Box::new(Pnpm::new()?),
        SourceKind::Yarn => Box::new(Yarn::new()?),
        SourceKind::Pipx => Box::new(Pipx::new()?),
        SourceKind::Uv => Box::new(Uv::new()?),
        SourceKind::Scoop => Box::new(Scoop::new()?),
        SourceKind::Cargo => Box::new(Cargo::new()?),
        SourceKind::Go => Box::new(Go::new()?),
        SourceKind::Dotnet => Box::new(Dotnet::new()?),
        SourceKind::PsGallery => Box::new(PsGallery::new()?),
        SourceKind::Gem => Box::new(Gem::new()?),
        SourceKind::Winget => Box::new(Winget::new()?),
        SourceKind::Choco => Box::new(Choco::new()?),
    })
}

pub(crate) fn find_on_path(name: &str) -> Option<PathBuf> {
    find_in(&std::env::var_os("PATH")?, name)
}

fn find_in(path: &std::ffi::OsStr, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|dir| executable_in(&dir, name))
}

#[cfg(windows)]
fn executable_in(dir: &Path, name: &str) -> Option<PathBuf> {
    const EXTENSIONS: [&str; 3] = ["exe", "cmd", "bat"];

    let bare = dir.join(name);
    if Path::new(name).extension().is_some() {
        return bare.is_file().then_some(bare);
    }
    EXTENSIONS
        .iter()
        .map(|extension| bare.with_extension(extension))
        .find(|candidate| candidate.is_file())
}

#[cfg(not(windows))]
fn executable_in(dir: &Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let candidate = dir.join(name);
    let metadata = fs::metadata(&candidate).ok()?;
    (metadata.is_file() && metadata.permissions().mode() & 0o111 != 0).then_some(candidate)
}

pub(crate) fn manifest_names(raw: &str) -> Result<Vec<String>> {
    let manifest: GlobalManifest = serde_json::from_str(raw)?;
    Ok(manifest.dependencies.into_keys().collect())
}

pub(crate) fn node_modules_listing(root: &Path, source: SourceKind) -> Vec<Installed> {
    let Ok(raw) = fs::read_to_string(root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(names) = manifest_names(&raw) else {
        return Vec::new();
    };
    names
        .iter()
        .filter_map(|name| installed_version(root, name, source))
        .collect()
}

pub(crate) fn installed_version(root: &Path, name: &str, source: SourceKind) -> Option<Installed> {
    let manifest = root.join("node_modules").join(name).join("package.json");
    let raw = fs::read_to_string(manifest).ok()?;
    let installed: InstalledManifest = serde_json::from_str(&raw).ok()?;
    semver::Version::parse(&installed.version).ok()?;
    Some(Installed::new(name, installed.version, source))
}

#[cfg(windows)]
pub(crate) fn hidden_command(program: &Path) -> Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(windows))]
pub(crate) fn hidden_command(program: &Path) -> Command {
    Command::new(program)
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
