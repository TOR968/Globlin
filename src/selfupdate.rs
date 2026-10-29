use minisign_verify::{PublicKey, Signature};
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ureq::Agent;

use crate::{diagnostics, install, platform, Result};

const X64_ASSET: &str = "globlin.exe";
const ARM64_ASSET: &str = "globlin-arm64.exe";
const MACOS_ASSET: &str = "globlin-macos-apple-silicon.zip";
const APPIMAGE_X64_ASSET: &str = "globlin-x86_64.AppImage";
const APPIMAGE_ARM64_ASSET: &str = "globlin-aarch64.AppImage";
const SUMS_ASSET: &str = "SHA256SUMS";
const SUMS_SIGNATURE_ASSET: &str = "SHA256SUMS.minisig";
const BUNDLE_STAGING: &str = "update";
const RELEASE_PUBLIC_KEY: &str = "RWTTLQhyI4Bn/MclkJqsthxavfiFPGVos6b7/d9keFthKYcrAmvZ31V+";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub asset_url: String,
    pub sums_url: String,
    pub sums_signature_url: String,
}

fn offer(location: &str, current: &Version, asset: &str) -> Option<Release> {
    let (_, tag) = location.rsplit_once(TAG_PATH)?;
    let version = Version::parse(tag.trim_start_matches('v')).ok()?;
    if version <= *current {
        return None;
    }
    let url = |name: &str| format!("{DOWNLOAD_URL}/{tag}/{name}");
    Some(Release {
        version,
        asset_url: url(asset),
        sums_url: url(SUMS_ASSET),
        sums_signature_url: url(SUMS_SIGNATURE_ASSET),
    })
}

pub fn should_auto_apply(release: &Version, auto_update: bool, blocked: Option<&Version>) -> bool {
    auto_update && blocked != Some(release)
}

pub fn supersedes(release: &Version, installed: Option<&Version>) -> bool {
    installed.is_none_or(|installed| release > installed)
}

fn asset_for(os: &str, arch: &str, appimage: bool) -> Option<&'static str> {
    match (os, arch, appimage) {
        ("windows", "x86_64", _) => Some(X64_ASSET),
        ("windows", "aarch64", _) => Some(ARM64_ASSET),
        ("macos", "aarch64", _) => Some(MACOS_ASSET),
        ("linux", "x86_64", true) => Some(APPIMAGE_X64_ASSET),
        ("linux", "aarch64", true) => Some(APPIMAGE_ARM64_ASSET),
        _ => None,
    }
}

fn verify_signature(bytes: &[u8], signature: &str, public_key: &str) -> Result<()> {
    let key = PublicKey::from_base64(public_key)
        .map_err(|error| format!("the embedded release key is unusable: {error}"))?;
    let signature = Signature::decode(signature)
        .map_err(|error| format!("the release signature is unreadable: {error}"))?;
    key.verify(bytes, &signature, false)
        .map_err(|error| format!("the release signature does not match: {error}").into())
}

fn running_asset() -> Result<&'static str> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    asset_for(os, arch, platform::appimage().is_some())
        .ok_or_else(|| format!("no self-updating build is published for {os} {arch}").into())
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
    if let Ok(program) = platform::program_path() {
        remove_leftovers(&program);
    }
    if let Some(bundle) = std::env::current_exe().ok().and_then(|exe| bundle_of(&exe)) {
        fs::remove_dir_all(previous_path(&bundle)).ok();
        fs::remove_dir_all(sibling(&bundle, BUNDLE_STAGING)).ok();
    }
}

fn remove_leftovers(current: &Path) {
    fs::remove_file(previous_path(current)).ok();
    fs::remove_file(staged_path(current)).ok();
}

const LATEST_URL: &str = "https://github.com/TOR968/globlin/releases/latest";
const DOWNLOAD_URL: &str = "https://github.com/TOR968/globlin/releases/download";
const TAG_PATH: &str = "/releases/tag/";
const USER_AGENT: &str = "globlin";
const TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_mins(2);
pub const RESTART_FLAG: &str = "--replaced";

pub fn latest() -> Result<Option<Release>> {
    let asset = running_asset()?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let response = agent(TIMEOUT)
        .get(LATEST_URL)
        .header("User-Agent", USER_AGENT)
        .config()
        .max_redirects(0)
        .build()
        .call()?;
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .ok_or("the latest-release page did not redirect anywhere")?;
    Ok(offer(location, &current, asset))
}

pub fn apply(release: &Release) -> Result<Version> {
    clean_stale();
    let asset = running_asset()?;
    let download = get(&agent(DOWNLOAD_TIMEOUT), &release.asset_url)?.read_to_vec()?;
    let sums = get(&agent(TIMEOUT), &release.sums_url)?.read_to_string()?;
    let signature = get(&agent(TIMEOUT), &release.sums_signature_url)?.read_to_string()?;
    verify_signature(sums.as_bytes(), &signature, RELEASE_PUBLIC_KEY)?;
    verify(&download, &sums, asset)?;
    if is_bundle_archive(asset) {
        install_bundle(&download)?;
    } else {
        install_binary(&download)?;
    }
    record_installed_version(&release.version);
    Ok(release.version.clone())
}

fn is_bundle_archive(asset: &str) -> bool {
    Path::new(asset)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn install_binary(binary: &[u8]) -> Result<()> {
    let current = platform::program_path()?;
    let staged = staged_path(&current);
    fs::write(&staged, binary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
    }
    swap(&current, &staged).inspect_err(|_| {
        if should_discard_staged(&current) {
            fs::remove_file(&staged).ok();
        }
    })
}

#[cfg(target_os = "macos")]
fn install_bundle(archive: &[u8]) -> Result<()> {
    let current = std::env::current_exe()?;
    let bundle = bundle_of(&current).ok_or("Globlin is not running from inside a .app bundle")?;
    let staging = sibling(&bundle, BUNDLE_STAGING);
    fs::remove_dir_all(&staging).ok();
    fs::create_dir_all(&staging)?;
    let archive_path = staging.join(MACOS_ASSET);
    fs::write(&archive_path, archive)?;
    let unpacked = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&archive_path)
        .arg(&staging)
        .status()?;
    if !unpacked.success() {
        fs::remove_dir_all(&staging).ok();
        return Err(format!("ditto could not unpack the update: {unpacked}").into());
    }
    let staged = unpacked_bundle(&staging)?;
    let swapped = swap(&bundle, &staged);
    fs::remove_dir_all(&staging).ok();
    swapped
}

#[cfg(not(target_os = "macos"))]
fn install_bundle(_archive: &[u8]) -> Result<()> {
    Err("only the macOS build updates itself from an app bundle".into())
}

#[cfg(target_os = "macos")]
fn unpacked_bundle(staging: &Path) -> Result<PathBuf> {
    fs::read_dir(staging)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| "the update archive holds no .app bundle".into())
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn bundle_of(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let named = |path: &Path, name: &str| path.file_name().is_some_and(|file| file == name);
    let is_app = bundle
        .extension()
        .is_some_and(|extension| extension == "app");
    (named(macos, "MacOS") && named(contents, "Contents") && is_app).then(|| bundle.to_path_buf())
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
    Command::new(platform::program_path()?)
        .arg(RESTART_FLAG)
        .spawn()?;
    Ok(())
}

fn agent(timeout: Duration) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(timeout))
        .https_only(true)
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
