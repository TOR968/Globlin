use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ureq::Agent;

use crate::{diagnostics, install, platform, Result};

const X64_ASSET: &str = "globlin.exe";
const ARM64_ASSET: &str = "globlin-arm64.exe";
const MACOS_ARM64_ASSET: &str = "globlin-macos-arm64";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub exe_url: String,
    pub sha_url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

fn offer(body: &str, current: &Version, asset: &str) -> Option<Release> {
    let release: ApiRelease = serde_json::from_str(body).ok()?;
    let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
    if version <= *current {
        return None;
    }
    Some(Release {
        version,
        exe_url: asset_url(&release.assets, asset)?,
        sha_url: asset_url(&release.assets, &checksum_asset(asset))?,
    })
}

pub fn should_auto_apply(release: &Version, auto_update: bool, blocked: Option<&Version>) -> bool {
    auto_update && blocked != Some(release)
}

pub fn supersedes(release: &Version, installed: Option<&Version>) -> bool {
    installed.is_none_or(|installed| release > installed)
}

fn asset_url(assets: &[ApiAsset], name: &str) -> Option<String> {
    assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.clone())
}

fn asset_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some(X64_ASSET),
        ("windows", "aarch64") => Some(ARM64_ASSET),
        ("macos", "aarch64") => Some(MACOS_ARM64_ASSET),
        _ => None,
    }
}

fn checksum_asset(asset: &str) -> String {
    format!("{asset}.sha256")
}

fn running_asset() -> Result<&'static str> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    asset_for(os, arch).ok_or_else(|| format!("no build is published for {os} {arch}").into())
}

fn published_hash(body: &str, asset: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let (hash, name) = line.trim().split_once(char::is_whitespace)?;
        if name.trim() != asset || hash.len() != 64 {
            return None;
        }
        if !hash.chars().all(|character| character.is_ascii_hexdigit()) {
            return None;
        }
        Some(hash.to_ascii_lowercase())
    })
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    hash.iter().fold(String::with_capacity(64), |mut s, byte| {
        use std::fmt::Write;
        write!(s, "{byte:02x}").unwrap();
        s
    })
}

fn verify(bytes: &[u8], sha_body: &str, asset: &str) -> Result<()> {
    let Some(published) = published_hash(sha_body, asset) else {
        return Err("the release did not publish a usable checksum".into());
    };
    let actual = digest(bytes);
    if actual == published {
        return Ok(());
    }
    Err(format!("checksum mismatch: expected {published}, got {actual}").into())
}

fn staged_path(current: &Path) -> PathBuf {
    sibling(current, "new")
}

fn previous_path(current: &Path) -> PathBuf {
    sibling(current, "old")
}

fn sibling(current: &Path, suffix: &str) -> PathBuf {
    let mut name = current.as_os_str().to_os_string();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

fn swap(current: &Path, staged: &Path) -> Result<()> {
    let previous = previous_path(current);
    fs::rename(current, &previous)?;
    match fs::rename(staged, current) {
        Ok(()) => Ok(()),
        Err(error) => {
            match fs::rename(&previous, current) {
                Ok(()) => Err(error.into()),
                Err(rollback_error) => Err(format!(
                    "failed to activate new executable: {}; rollback also failed: {}; {} is now missing, but the previous build is intact at {} and renaming it back to {} restores it",
                    error, rollback_error, current.display(), previous.display(), current.display()
                ).into())
            }
        }
    }
}

pub fn clean_stale() {
    if let Ok(current) = std::env::current_exe() {
        remove_leftovers(&current);
    }
}

fn remove_leftovers(current: &Path) {
    fs::remove_file(previous_path(current)).ok();
    fs::remove_file(staged_path(current)).ok();
}

const LATEST_URL: &str = "https://api.github.com/repos/TOR968/globlin/releases/latest";
const USER_AGENT: &str = "globlin";
const TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_mins(2);
pub const RESTART_FLAG: &str = "--replaced";

pub fn latest() -> Result<Option<Release>> {
    let asset = running_asset()?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let body = get(&agent(TIMEOUT), LATEST_URL)?.read_to_string()?;
    Ok(offer(&body, &current, asset))
}

pub fn apply(release: &Release) -> Result<Version> {
    clean_stale();
    let binary = get(&agent(DOWNLOAD_TIMEOUT), &release.exe_url)?.read_to_vec()?;
    let checksum = get(&agent(TIMEOUT), &release.sha_url)?.read_to_string()?;
    verify(&binary, &checksum, running_asset()?)?;

    let current = std::env::current_exe()?;
    let staged = staged_path(&current);
    fs::write(&staged, &binary)?;
    match swap(&current, &staged) {
        Ok(()) => {
            record_installed_version(&release.version);
            Ok(release.version.clone())
        }
        Err(error) => {
            if should_discard_staged(&current) {
                fs::remove_file(&staged).ok();
            }
            Err(error)
        }
    }
}

fn record_installed_version(version: &Version) {
    if !install::setup_managed() {
        return;
    }
    if let Err(error) = platform::record_installed_version(&version.to_string()) {
        diagnostics::record_self_update_failure(&format!(
            "could not record {version} as the installed version: {error}\n"
        ));
    }
}

fn should_discard_staged(current: &Path) -> bool {
    current.exists()
}

pub fn relaunch() -> Result<()> {
    Command::new(std::env::current_exe()?)
        .arg(RESTART_FLAG)
        .spawn()?;
    Ok(())
}

fn agent(timeout: Duration) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

fn get(agent: &Agent, url: &str) -> Result<ureq::Body> {
    Ok(agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .into_body())
}

#[cfg(test)]
mod tests;
