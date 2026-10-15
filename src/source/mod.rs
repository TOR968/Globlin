use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Deserialize;

use crate::config::Config;
use crate::model::{BlockedScript, Installed, Refusal, SourceKind, KINDS};
use crate::process;
use crate::Result;

mod brew;
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

pub use brew::Brew;
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

    fn update_to(&self, name: &str, _version: &str) -> Option<Command> {
        self.update_command(name)
    }

    fn update_together(&self, _names: &[&str]) -> Option<Command> {
        None
    }

    fn uninstall_command(&self, name: &str) -> Option<Command>;

    fn remove_directly(&self, _name: &str) -> Option<Result<()>> {
        None
    }

    fn blocked_scripts(&self, _stderr: &str) -> Option<Vec<BlockedScript>> {
        None
    }

    fn refusal(&self, _output: &Output) -> Option<Refusal> {
        None
    }

    fn approve_scripts(&self, _scripts: &[BlockedScript]) -> Result<()> {
        Err(format!("{} does not gate install scripts", self.kind().label()).into())
    }
}

#[derive(Debug)]
pub struct NotInstalled(String);

impl std::fmt::Display for NotInstalled {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NotInstalled {}

pub(crate) fn not_on_path(program: &str) -> crate::Error {
    not_installed(format!("{program} was not found on PATH"))
}

pub(crate) fn not_installed(message: String) -> crate::Error {
    Box::new(NotInstalled(message))
}

pub fn enabled(config: &Config) -> Result<Vec<Box<dyn PackageSource>>> {
    assemble(
        KINDS
            .into_iter()
            .filter(|kind| kind.available() && config.source_enabled(*kind))
            .map(|kind| build(kind, config)),
    )
}

fn assemble(
    built: impl IntoIterator<Item = Result<Box<dyn PackageSource>>>,
) -> Result<Vec<Box<dyn PackageSource>>> {
    let mut sources: Vec<Box<dyn PackageSource>> = Vec::new();
    let mut failures = Vec::new();
    let mut attempted = false;

    for result in built {
        attempted = true;
        match result {
            Ok(source) => sources.push(source),
            Err(error) if error.is::<NotInstalled>() => {}
            Err(error) => failures.push(error.to_string()),
        }
    }

    if !attempted {
        return Err("no package sources are enabled in the config".into());
    }
    if sources.is_empty() && !failures.is_empty() {
        return Err(failures.join("; ").into());
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
        SourceKind::Brew => Box::new(Brew::new()?),
        SourceKind::Winget => Box::new(Winget::new()?),
        SourceKind::Choco => Box::new(Choco::new()?),
    })
}

pub(crate) fn source_for<'a>(
    sources: &'a [Box<dyn PackageSource>],
    name: &str,
    kind: SourceKind,
) -> std::result::Result<&'a dyn PackageSource, String> {
    sources
        .iter()
        .find(|source| source.kind() == kind)
        .map(AsRef::as_ref)
        .ok_or_else(|| format!("{name}: the {} source is not available\n", kind.label()))
}

pub(crate) fn run_change(
    command: Option<Command>,
    name: &str,
    kind: SourceKind,
) -> std::result::Result<Output, String> {
    let mut command =
        command.ok_or_else(|| format!("{name}: {} has no command for this\n", kind.label()))?;
    process::output_within(&mut command, process::CHANGE_LIMIT)
        .map_err(|error| format!("{name}: could not run {}: {error}\n", kind.label()))
}

pub(crate) fn run_listing(command: &mut Command) -> Result<Output> {
    Ok(process::output_within(command, process::LISTING_LIMIT)?)
}

pub(crate) fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn listed<T>(rows: Vec<T>, command: &str, output: &Output) -> Result<Vec<T>> {
    if rows.is_empty() && !output.status.success() {
        return Err(silent_listing(command, output).into());
    }
    Ok(rows)
}

pub(crate) fn silent_listing(command: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("{command} listed nothing and {}", output.status)
    } else {
        format!("{command} listed nothing and {}: {stderr}", output.status)
    }
}

pub(crate) fn find_on_path(name: &str) -> Option<PathBuf> {
    find_in(&std::env::var_os("PATH")?, name)
}

fn find_in(path: &std::ffi::OsStr, name: &str) -> Option<PathBuf> {
    path_entries(path)
        .into_iter()
        .find_map(|dir| executable_in(&dir, name))
}

#[cfg(windows)]
fn path_entries(path: &std::ffi::OsStr) -> Vec<PathBuf> {
    path.to_string_lossy()
        .split(';')
        .map(|entry| entry.replace('"', ""))
        .filter(|entry| !entry.trim().is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn path_entries(path: &std::ffi::OsStr) -> Vec<PathBuf> {
    std::env::split_paths(path).collect()
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
    Ok(manifest_specs(raw)?.into_keys().collect())
}

fn manifest_specs(raw: &str) -> Result<BTreeMap<String, String>> {
    let manifest: GlobalManifest = serde_json::from_str(raw)?;
    Ok(manifest.dependencies)
}

pub(crate) fn node_modules_listing(root: &Path, source: SourceKind) -> Vec<Installed> {
    let Ok(raw) = fs::read_to_string(root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(specs) = manifest_specs(&raw) else {
        return Vec::new();
    };
    specs
        .iter()
        .filter_map(|(name, spec)| {
            Some(installed_version(root, name, source)?.with_external(names_another_origin(spec)))
        })
        .collect()
}

pub(crate) fn names_another_origin(spec: &str) -> bool {
    spec.contains(':') || spec.contains('/')
}

pub(crate) fn plain_name(name: &str) -> Option<&str> {
    let plain = !name.is_empty()
        && name
            .chars()
            .all(|glyph| glyph.is_ascii_alphanumeric() || matches!(glyph, '.' | '_' | '-'));
    plain.then_some(name)
}

pub(crate) fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

pub(crate) fn pinned_to(name: &str, version: &str) -> String {
    format!("{name}@{version}")
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
